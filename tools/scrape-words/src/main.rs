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
extern crate unicode_normalization;
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
  collections::{BTreeSet, HashMap},
  env,
  fs::File,
  io::{self, BufReader, BufWriter, Write},
  ops::DerefMut,
  string,
  sync::{Arc, Mutex},
};
use unicode_normalization::UnicodeNormalization;

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
    FromUtf8(string::FromUtf8Error);
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
  static ref DROP_RE: Regex =
    Regex::new(r"^[\p{N}\p{M}\p{P}\p{Z}\p{C}]*$").unwrap();
  // static ref NONWORD_RE: Regex = Regex::new(r"[\W--\s]+").unwrap();
  static ref NONWORD_TRIM_RE: Regex =
    Regex::new(r"(^[\W--\s]+|[\W--\s]+$)").unwrap();
  static ref NONWORD_STRIP_RE: Regex =
    Regex::new(r"[\p{M}\p{Ps}\p{Pe}\p{Pi}\p{Pf}\p{Po}\p{C}--/]").unwrap();
  static ref DEFER_SPLIT_RE: Regex =
    Regex::new(r"[/\p{Pc}\p{Pd}\p{Z}]").unwrap();
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

fn postprocess_words(outs: Vec<u8>, outf: &str) -> Result<()> {
  writeln!(io::stderr(), "post-processing text...")?;

  writeln!(io::stderr(), "  decoding UTF-8...")?;
  let mut string = String::from_utf8(outs)?;

  writeln!(io::stderr(), "  applying NFKD decomp...")?;
  string = string.nfkd().collect();

  writeln!(io::stderr(), "  converting to lowercase...")?;
  string = string.to_lowercase();

  writeln!(io::stderr(), "  normalizing whitespace...")?;
  string = WHITESPACE_RE.replace_all(&string, " ").into_owned();

  let mut forms: HashMap<String, BTreeSet<String>> = HashMap::new();
  let mut counts: HashMap<String, usize> = HashMap::new();
  let mut defer: HashMap<String, usize> = HashMap::new();

  writeln!(io::stderr(), "  performing frequency analysis...")?;

  for word in WHITESPACE_RE.split(&string) {
    use std::collections::hash_map::Entry::*;

    let mut norm = NONWORD_TRIM_RE.replace_all(word, "").into_owned();
    norm = NONWORD_STRIP_RE.replace_all(&norm, "").into_owned();

    match forms.entry(norm.clone()) {
      Vacant(v) => {
        v.insert(BTreeSet::new()).insert(word.into());
      }
      Occupied(mut o) => {
        o.get_mut().insert(word.into());
      }
    }

    if norm.len() == 0 {
      continue;
    }

    if DROP_RE.is_match(&norm) {
      writeln!(io::stdout(), "  DROP {:?}", norm)?;

      continue;
    }

    if DEFER_SPLIT_RE.is_match(&norm) {
      match defer.entry(norm.into()) {
        Vacant(v) => {
          writeln!(io::stdout(), "    deferring {:?}", v.key())?;
          v.insert(1);
        }
        Occupied(mut o) => {
          let v = o.get_mut();
          *v = *v + 1;
        }
      }
    } else {
      match counts.entry(norm.into()) {
        Vacant(v) => {
          v.insert(1);
        }
        Occupied(mut o) => {
          let v = o.get_mut();
          *v = *v + 1;
        }
      }
    }
  }

  writeln!(io::stderr(), "  processing deferred words...")?;

  let mut defer_counts: HashMap<String, usize> = HashMap::new();

  {
    #[derive(Debug)]
    enum Action {
      Split,
      Leave,
      Reintroduce,
    }

    let mut actions: HashMap<String, Action> = HashMap::new();

    for (word, count) in defer.iter() {
      use std::collections::hash_map::Entry::*;

      let count = *count;

      let split: Vec<_> = DEFER_SPLIT_RE.split(&word).collect();

      // TODO: remove duplicate code

      if split.iter().all(|e| {
        let e = e.to_string(); // TODO: don't clone this
        counts.get(&e).unwrap_or(&0) + defer.get(&e).unwrap_or(&0) >= count
      }) {
        actions.insert(word.clone(), Action::Split);

        for word in split {
          match defer_counts.entry(word.into()) {
            Vacant(v) => {
              v.insert(count);
            }
            Occupied(mut o) => {
              let v = o.get_mut();
              *v = *v + count;
            }
          }
        }
      } else {
        actions.insert(word.clone(), Action::Leave);

        match defer_counts.entry(word.clone()) {
          Vacant(v) => {
            v.insert(count);
          }
          Occupied(mut o) => {
            let v = o.get_mut();
            *v = *v + count;
          }
        }
      }
    }

    let mut sorted: Vec<_> = actions.iter().collect();

    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (word, action) in sorted {
      writeln!(io::stdout(), "{:16} {}", format!("{:?}", action), word)?;
    }
  }

  writeln!(io::stderr(), "  collating deferred words...")?;

  for (word, count) in defer_counts {
    use std::collections::hash_map::Entry::*;

    match counts.entry(word.into()) {
      Vacant(v) => {
        v.insert(count);
      }
      Occupied(mut o) => {
        let v = o.get_mut();
        *v = *v + count;
      }
    }
  }

  writeln!(io::stderr(), "  finalizing...")?;

  let mut sorted: Vec<_> = counts.iter().collect();

  sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

  writeln!(io::stderr(), "printing data...")?;

  let mut outs = File::create(outf)?;

  for (i, (word, count)) in sorted.iter().enumerate() {
    writeln!(outs, "#{0:6} ({1:6}) : {2:32} ({2:?})", i + 1, count, word)?;

    if let Some(f) = forms.get(*word) {
      for form in f.iter().filter(|f| f != word) {
        writeln!(outs, " -- {0:32} ({0:?})", form)?;
      }
    }
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
  let pretty_outf = args[4].clone();
  let outf = args[5].clone();

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

            let outs = Arc::new(Mutex::new(Some(Vec::<u8>::new())));

            let mut pretty_outs = File::create(pretty_outf).unwrap();

            let outs_1 = outs.clone();

            stream::futures_ordered(tasks)
              .collect()
              .from_err()
              .map(move |vec| {
                writeln!(io::stderr(), "\nprocessing data...").unwrap();

                let mut outs = outs.lock().unwrap();
                let outs = outs.as_mut().unwrap();

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
                let outs = outs_1;
                let mut outs = outs.lock().unwrap();
                let outs = outs.take().unwrap();

                postprocess_words(outs, &outf).unwrap();
              })
          })
      })
      .map_err(|e: Error| {
        writeln!(io::stderr(), "an error occurred: {}", e).unwrap();
      }),
  );

  Ok(())
}
