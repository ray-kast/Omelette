extern crate base64;
extern crate futures;
extern crate html5ever;
extern crate http;
extern crate hyper;
extern crate hyper_tls;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate serde_value;
extern crate tokio;
extern crate url;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

mod future_semaphore;
mod process_html;
mod reddit;
mod thread_future;

use futures::{future, prelude::*, stream};
use reddit::{
  auth::{self, AuthToken},
  client, read, request, types, AppDuration, AppInfo,
};
use regex::Regex;
use std::{
  cmp,
  collections::HashMap,
  env,
  fs::File,
  io::{self, BufReader, BufWriter, Write},
  ops::DerefMut,
  sync::{Arc, Mutex},
};

// TODO: I should probably consolidate reddit::*::Error
error_chain! {
  links {
    Auth(auth::Error, auth::ErrorKind);
    Client(client::Error, client::ErrorKind);
    ProcessHtml(process_html::Error, process_html::ErrorKind);
    Read(read::Error, read::ErrorKind);
    Request(request::Error, request::ErrorKind);
    Types(types::Error, types::ErrorKind);
  }

  foreign_links {
    Io(io::Error);
    Serde(serde_json::Error);
  }
}

struct ClientHooks;

impl client::ClientHooks for ClientHooks {
  fn auth_code_state(&self) -> String {
    "uwu".into()
  }

  fn save_token(
    &self,
    tok: auth::RcAuthToken,
  ) -> Box<dyn Future<Item = (), Error = ()> + Send> {
    Box::new(
      File::create("etc/apitok.json")
        .into_future()
        .from_err()
        .and_then(move |file| {
          let file = BufWriter::new(file);

          writeln!(io::stderr(), "saving etc/apitok.json...").unwrap();

          serde_json::to_writer(file, tok.as_ref())
            .into_future()
            .from_err()
        })
        .map_err(|e: Error| {
          writeln!(io::stderr(), "failed to save etc/apitok.json: {}", e)
            .unwrap()
        }),
    )
  }
}

lazy_static! {
  static ref NEWLINE_RE: Regex = Regex::new(r"(\n+)").unwrap();
  static ref WHITESPACE_RE: Regex = Regex::new(r"\s+").unwrap();
  static ref NONWORD_RE: Regex = Regex::new(r"[\W--\s]+").unwrap();
}

fn get_body_string(html: &str) -> Result<String> {
  let body = process_html::unwrap(html)?;

  Ok(process_html::pretty_unwrap(&body)?)
}

fn print_link<W>(link: &types::Link, outs: &mut W) -> Result<()>
where
  W: Write,
{
  writeln!(outs, "## ({}) {} ({})", link.score, link.title, link.url)?;

  writeln!(outs, "  ## u/{}", link.author)?;

  let body = link.selftext_html.clone().unwrap_or("".into());
  let body = get_body_string(&body)?;

  if body.len() != 0 {
    let body = NEWLINE_RE.replace_all(&body, "$1  # ");

    writeln!(outs, "  # {}", body)?;
  }

  Ok(())
}

fn print_comment<W>(
  comment: &types::Comment,
  indent: &str,
  outs: &mut W,
) -> Result<()>
where
  W: Write,
{
  let body = &comment.body_html;
  let body = get_body_string(&body).unwrap();

  if NEWLINE_RE.is_match(&body) {
    let body = format!("\n{}", body);
    let body =
      NEWLINE_RE.replace_all(&body, format!("$1{}  : ", indent).as_str());

    writeln!(
      outs,
      "{}({:4}) u/{} ->{}",
      indent, comment.score, comment.author, body,
    )?;
  } else {
    writeln!(
      outs,
      "{}({:4}) u/{:32}: {}",
      indent, comment.score, comment.author, body,
    )?;
  }

  match &comment.replies {
    types::CommentReplies::Some(l) => {
      let l = l.as_listing();
      for reply in &l.children {
        match reply {
          types::Thing::Comment(c) => {
            print_comment(&c, &format!("{}  ", indent), outs)?
          }
          types::Thing::More(_) => writeln!(outs, "{}  ...", indent)?,
          _ => writeln!(outs, "{}<unexpected Thing>", indent)?,
        }
      }
    }
    types::CommentReplies::None => {}
  }

  Ok(())
}

// TODO: handle flair?

fn dump_link<W>(link: &types::Link, outs: &mut W) -> Result<()>
where
  W: Write,
{
  let body = link.selftext_html.clone().unwrap_or("".into());
  let body = get_body_string(&body).unwrap();

  writeln!(outs, "{}", body)?;

  Ok(())
}

fn dump_comment<W>(comment: &types::Comment, outs: &mut W) -> Result<()>
where
  W: Write,
{
  let body = &comment.body_html;
  let body = get_body_string(&body).unwrap();

  writeln!(outs, "{}", body)?;

  match &comment.replies {
    types::CommentReplies::Some(l) => {
      let l = l.as_listing();
      for reply in &l.children {
        match reply {
          types::Thing::Comment(c) => dump_comment(&c, outs)?,
          types::Thing::More(_) => (),
          _ => writeln!(outs, "<unexpected Thing>")?,
        }
      }
    }
    types::CommentReplies::None => (),
  }

  Ok(())
}

fn main() -> Result<()> {
  let id = {
    let file = File::open("etc/apikey.json")?;

    serde_json::from_reader(file)?
  };

  let app = AppInfo::new_rc(
    id,
    "http://rk1024.net/".parse().unwrap(),
    AppDuration::Permanent,
    "read".split(" "),
    "linux",
    "rk1024/scrape-words",
    "v0.3.0",
    "rookie1024",
  );

  fn retrieve_token() -> Result<AuthToken> {
    let file = File::open("etc/apitok.json")?;
    let file = BufReader::new(file);

    Ok(serde_json::from_reader(file)?)
  }

  let tok = match retrieve_token() {
    Ok(tok) => Some(tok),
    Err(e) => {
      writeln!(io::stderr(), "failed to get saved token: {}", e).unwrap();
      None
    }
  };

  writeln!(io::stderr(), "initializing...").unwrap();

  let args: Vec<_> = env::args().collect();

  let subreddit = args[1].clone();
  let sort: read::SortType = args[2].parse().unwrap();
  let limit: u32 = args[3].parse().unwrap();
  let outf = args[4].clone();

  tokio::run(
    client::Client::new(Arc::new(ClientHooks), app, tok, 10)
      .from_err()
      .and_then(move |client| {
        writeln!(io::stderr(), "listing r/{}...", subreddit).unwrap();

        let listings = Arc::new(Mutex::new(Vec::<types::Listing>::new()));

        let client_1 = client.clone();
        let listings_1 = listings.clone();

        future::loop_fn((limit, None), move |(remain, after)| {
          let limit = cmp::min(remain, 100);
          let client = client_1.clone();
          let listings = listings_1.clone();

          writeln!(io::stderr(), "taking {} of {}", limit, remain).unwrap();

          read::list_subreddit(
            client.clone(),
            subreddit.clone(),
            sort,
            Some(limit),
            after,
          ).map(move |listing| {
            let mut listings = listings.lock().unwrap();

            let len = listing.children.len();

            if len > 0 {
              let name = listing.children[len - 1].as_link().name.clone();

              listings.push(listing);

              let remain = remain - limit;

              if remain > 0 {
                future::Loop::Continue((remain, Some(name)))
              } else {
                future::Loop::Break(())
              }
            } else {
              writeln!(io::stderr(), "stopping on empty listing").unwrap();
              future::Loop::Break(())
            }
          })
        }).from_err()
          .and_then(move |_| {
            let links: Vec<_>;

            {
              let mut listings = listings.lock().unwrap();
              links = listings.drain(0..).flat_map(|l| l.children).collect();
            }

            writeln!(io::stderr(), "retrieving comments...").unwrap();

            let total = links.len();

            let n = Arc::new(Mutex::new(0));

            let tasks = links.iter().map(|child| {
              let n = n.clone();

              read::get_comments(client.clone(), child.as_link()).map(
                move |(links, comments)| {
                  {
                    let mut n = n.lock().unwrap();
                    *n = *n + 1;

                    for link in &links.as_listing().children {
                      write!(
                        io::stderr(),
                        "\r\x1b[2K  ({:4}/{:4}) {}",
                        n,
                        total,
                        link.as_link().url
                      ).unwrap();
                      io::stderr().flush().unwrap();
                    }
                  }

                  (links, comments)
                },
              )
            });

            let outs = Arc::new(Mutex::new(Vec::<u8>::new()));

            let mut pretty_outs = File::create(outf).unwrap();

            let outs_1 = outs.clone();

            stream::futures_ordered(tasks)
              .collect()
              .from_err()
              .map(move |vec| {
                writeln!(io::stderr(), "\nprocessing data...").unwrap();

                let mut outs = outs.lock().unwrap();
                let outs = outs.deref_mut(); // TODO: is there any way to do away with this?

                let total = vec.len();

                for (i, (links, comments)) in vec.iter().enumerate() {
                  for link in &links.as_listing().children {
                    write!(
                      io::stderr(),
                      "\r\x1b[2K  ({:4}/{:4}) {}",
                      i + 1,
                      total,
                      link.as_link().url
                    ).unwrap();
                    io::stderr().flush().unwrap();

                    match link {
                      types::Thing::Link(l) => {
                        print_link(&l, &mut pretty_outs).unwrap();
                        dump_link(&l, outs).unwrap();
                      }
                      _ => {
                        writeln!(pretty_outs, "## <unexpected Thing>").unwrap()
                      }
                    }
                  }

                  for comment in &comments.as_listing().children {
                    match comment {
                      types::Thing::Comment(c) => {
                        print_comment(&c, "  ", &mut pretty_outs).unwrap();
                        dump_comment(&c, outs).unwrap();
                      }
                      types::Thing::More(_) => {
                        writeln!(pretty_outs, "  ...").unwrap()
                      }
                      _ => {
                        writeln!(pretty_outs, "  <unexpected Thing>").unwrap()
                      }
                    }
                  }

                  write!(pretty_outs, "\n\n\n").unwrap();
                }

                writeln!(io::stderr(), "").unwrap();
              })
              .map(move |_| {
                writeln!(io::stderr(), "post-processing text...").unwrap();

                let outs = outs_1;
                let mut outs = outs.lock().unwrap();
                // TODO: STOP ABUSING INTERNAL MUTABILITY
                let outs = outs.deref_mut();

                // TODO: DON'T FUCKING CLONE THIS
                let outs: Vec<_> = outs.clone();

                writeln!(io::stderr(), "  decoding UTF-8...").unwrap();

                let mut string = String::from_utf8(outs).unwrap();

                // TODO: apply a Unicode decomp transform

                writeln!(io::stderr(), "  stripping nonword chars...").unwrap();

                string = NONWORD_RE.replace_all(&string, "").into_owned();

                writeln!(io::stderr(), "  converting to lowercase...").unwrap();

                string = string.to_lowercase();

                writeln!(io::stderr(), "  normalizing whitespace...").unwrap();

                string = WHITESPACE_RE.replace_all(&string, " ").into_owned();

                let mut counts: HashMap<String, usize> = HashMap::new();

                writeln!(io::stderr(), "  performing frequency analysis...")
                  .unwrap();

                for word in WHITESPACE_RE.split(&string) {
                  use std::collections::hash_map::Entry::*;

                  match counts.entry(word.into()) {
                    Vacant(v) => {
                      v.insert(1);
                    }
                    Occupied(mut o) => {
                      let v = o.get_mut();
                      *v = *v + 1;
                    }
                  }
                }

                writeln!(io::stderr(), "  finalizing...").unwrap();

                let mut sorted: Vec<_> = counts.iter().collect();

                sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

                writeln!(io::stderr(), "printing data...").unwrap();

                for (i, (word, count)) in sorted.iter().enumerate() {
                  println!("#{:6} ({:6}) : {}", i + 1, count, word);
                }
              })
          })
      })
      .map_err(|e: Error| {
        writeln!(io::stderr(), "an error occurred: {}", e).unwrap();
      }),
  );

  Ok(())
}
