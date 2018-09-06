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

mod process_html;
mod reddit;
mod thread_future;

use futures::{stream, Future, IntoFuture, Stream};
use reddit::{
  auth::{self, AuthToken},
  client, read, request, types, AppDuration, AppInfo,
};
use regex::Regex;
use std::{
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
    "v0.2.0",
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
  let limit: u32 = args[2].parse().unwrap();
  let pretty: bool = args[3].parse().unwrap();

  tokio::run(
    client::Client::new(Arc::new(ClientHooks), app, tok)
      .from_err()
      .and_then(move |client| {
        writeln!(io::stderr(), "listing r/{}...", subreddit).unwrap();

        read::list_subreddit(
          client.clone(),
          subreddit,
          read::SortType::Hot,
          Some(limit),
          None,
        ).from_err()
          .and_then(move |listing| {
            writeln!(io::stderr(), "retrieving comments...").unwrap();

            let tasks = listing
              .children
              .iter()
              .map(|child| read::get_comments(client.clone(), child.as_link()));

            let outs = Arc::new(Mutex::new(Vec::<u8>::new()));

            let outs_1 = outs.clone();

            stream::futures_ordered(tasks)
              .collect()
              .from_err()
              .map(move |vec| {
                let mut outs = outs.lock().unwrap();
                let outs = outs.deref_mut(); // TODO: is there any way to do away with this?

                for (links, comments) in vec.iter() {
                  for link in &links.as_listing().children {
                    match link {
                      types::Thing::Link(l) => if pretty {
                        print_link(&l, outs)
                      } else {
                        dump_link(&l, outs)
                      }.unwrap(),
                      _ => if pretty {
                        writeln!(outs, "## <unexpected Thing>").unwrap()
                      } else {
                        writeln!(outs, "<unexpected Thing>").unwrap()
                      },
                    }
                  }

                  for comment in &comments.as_listing().children {
                    match comment {
                      types::Thing::Comment(c) => if pretty {
                        print_comment(&c, "  ", outs)
                      } else {
                        dump_comment(&c, outs)
                      }.unwrap(),
                      types::Thing::More(_) => if pretty {
                        writeln!(outs, "  ...").unwrap()
                      },
                      _ => if pretty {
                        writeln!(outs, "  <unexpected Thing>").unwrap()
                      } else {
                        writeln!(outs, "<unexpected Thing>").unwrap()
                      },
                    }
                  }

                  if pretty {
                    write!(outs, "\n\n\n").unwrap();
                  }
                }
              })
              .map(move |_| {
                writeln!(io::stderr(), "printing data...").unwrap();

                let outs = outs_1;
                let mut outs = outs.lock().unwrap();
                // TODO: STOP ABUSING INTERNAL MUTABILITY
                let outs = outs.deref_mut();

                // TODO: DON'T FUCKING CLONE THIS
                let outs: Vec<_> = outs.clone();

                let string = String::from_utf8(outs).unwrap();

                if !pretty {
                  // TODO: apply a Unicode decomp transform

                  // let string = NONWORD_RE.replace_all(&string, "");

                  let string = WHITESPACE_RE.replace_all(&string, " ");
                }

                print!("{}", string);
              })
          })
      })
      .map_err(|e: Error| {
        writeln!(io::stderr(), "an error occurred: {}", e).unwrap();
      }),
  );

  Ok(())
}
