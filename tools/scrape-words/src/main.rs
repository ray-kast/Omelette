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

use futures::{future, Future, IntoFuture, Stream};
use reddit::{
  auth::{self, AuthToken},
  read, request, types, AppDuration, AppId, AppInfo,
};
use regex::Regex;
use std::{
  borrow::{Borrow, Cow},
  cell::RefCell,
  fs::File,
  io::{self, BufReader, BufWriter, Write},
  ops::DerefMut,
  string,
  sync::{Arc, Mutex},
};

error_chain! {
  links {
    Auth(auth::Error, auth::ErrorKind);
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

fn main() {
  let id: AppId;

  {
    let file =
      File::open("etc/apikey.json").expect("couldn't open apikey.json");
    let file = BufReader::new(file);

    id = serde_json::from_reader(file).expect("failed to parse apikey.json");
  }

  fn retrieve_token() -> Result<AuthToken> {
    let file = File::open("etc/apitok.json")?;
    let file = BufReader::new(file);

    Ok(serde_json::from_reader(file)?)
  }

  let tok = match retrieve_token() {
    Ok(tok) => Some(tok),
    Err(e) => {
      println!("failed to get saved token: {}", e);
      None
    }
  };

  let app = AppInfo::new_rc(
    id,
    "http://rk1024.net/".parse().unwrap(),
    AppDuration::Permanent,
    "read".split(" "),
    "linux",
    "rk1024/scrape-words",
    "v0.1.0",
    "rookie1024",
  );

  // println!("fucc: {:?}", {
  //   let file = File::open("fucc.json").unwrap();

  //   serde_json::from_reader::<_, types::Thing>(file)
  // });

  // println!("fucc2: {:?}", {
  //   let file = File::open("fucc2.json").unwrap();

  //   serde_json::from_reader::<_, types::Thing>(file)
  // });

  // println!("fucc3: {:?}", {
  //   let file = File::open("fucc3.json").unwrap();

  //   serde_json::from_reader::<_, types::Thing>(file)
  // });

  // println!("fucc4: {:?}", {
  //   let file = File::open("fucc4.json").unwrap();

  //   serde_json::from_reader::<_, (types::Thing, types::Thing)>(file)
  //     .map(|(a, b)| b)
  // });

  // return;

  lazy_static! {
    static ref NEWLINE_RE: Regex = Regex::new(r"(\n+)").unwrap();
    static ref WHITESPACE_RE: Regex = Regex::new(r"\s+").unwrap();
    static ref NONWORD_RE: Regex = Regex::new(r"[\W--\s]+").unwrap();
  }

  fn get_body_string(html: &str) -> Result<String> {
    let body = process_html::unwrap(html)?;

    // println!("{}", body);

    Ok(process_html::pretty_unwrap(&body)?)
  }

  fn print_link<W>(link: &types::Link, outs: &mut W) -> Result<()>
  where
    W: Write,
  {
    writeln!(outs, "## ({}) {} ({})", link.score, link.title, link.url)?;

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
        let l = l.clone().into_listing();
        for reply in l.children {
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
        let l = l.clone().into_listing();
        for reply in l.children {
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

  tokio::run(future::lazy(move || {
    let client = request::create_client_rc().unwrap();

    let outs = Arc::new(Mutex::new(Vec::<u8>::new()));

    let outs_1 = outs.clone();
    let outs_2 = outs.clone();

    let app_1 = app.clone();
    let app_2 = app.clone();
    let client_1 = client.clone();
    let client_2 = client.clone();

    let args = std::env::args().collect::<Vec<_>>();

    let subreddit = args[1].clone();
    let limit: u32 = args[2].parse().unwrap();
    let pretty: bool = args[3].parse().unwrap();

    writeln!(io::stderr(), "authenticating...").unwrap();

    auth::authenticate(app.clone(), client, tok, || "uwu")
      .from_err()
      .and_then(move |tok| {
        let tok_1 = tok.clone();
        let tok_2 = tok.clone();

        writeln!(io::stderr(), "saving apitok.json...").unwrap();

        File::create("etc/apitok.json")
          .into_future()
          .from_err()
          .and_then(|file| {
            let file = BufWriter::new(file);

            serde_json::to_writer(file, tok.as_ref())
              .into_future()
              .map(|_| tok)
              .from_err()
          })
          .and_then(move |_| {
            let app = app_1;
            let tok = tok_1;
            let client = client_1;

            writeln!(io::stderr(), "listing r/{}...", subreddit).unwrap();

            read::list_subreddit(
              app.clone(),
              tok.clone(),
              client.clone(),
              subreddit,
              // read::SortType::Top(read::SortRange::All),
              read::SortType::Hot,
              Some(limit), // TODO
              None,
            ).from_err()
          })
          .and_then(move |listing| {
            let app = app_2;
            let tok = tok_2;
            let client = client_2;

            writeln!(io::stderr(), "retrieving comments...").unwrap();

            let tasks = listing.children.iter().map(|child| {
              read::get_comments(
                app.clone(),
                tok.clone(),
                client.clone(),
                &child.clone().into_link(),
              )
            });

            futures::stream::futures_ordered(tasks)
              .collect()
              .from_err()
              .map(move |v| {
                let outs = outs_1;
                let mut outs = outs.lock().unwrap();
                let outs = outs.deref_mut();

                for (links, comments) in v.iter() {
                  for link in links.clone().into_listing().children.iter() {
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

                  for comment in comments.clone().into_listing().children.iter()
                  {
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
          })
          .and_then(|_| {
            writeln!(io::stderr(), "printing data...").unwrap();

            let outs = outs_2;
            let mut outs = outs.lock().unwrap();
            // TODO: STOP ABUSING INTERNAL MUTABILITY
            let outs = outs.deref_mut();

            // TODO: DON'T FUCKING CLONE THIS
            let outs: Vec<_> = outs.clone();

            let string = String::from_utf8(outs).unwrap();

            // TODO: apply a Unicode decomp transform

            // let string = NONWORD_RE.replace_all(&string, "");

            let string = WHITESPACE_RE.replace_all(&string, " ");

            print!("{}", string);

            Ok(()).into_future()
          })
      })
      .map_err(|e: Error| {
        writeln!(io::stderr(), "encountered an error: {}", e).unwrap()
      }) // TODO: make this less vague
  }));
}
