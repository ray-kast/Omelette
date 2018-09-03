extern crate base64;
extern crate futures;
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
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

mod reddit;

use futures::{future, Future, IntoFuture, Stream};
use reddit::{
  auth::{self, AuthToken},
  read, request, types, AppDuration, AppId, AppInfo,
};
use regex::Regex;
use std::{
  fs::File,
  io::{self, BufReader, BufWriter},
  string,
};

error_chain! {
  links {
    Auth(auth::Error, auth::ErrorKind);
    Read(read::Error, read::ErrorKind);
    Request(request::Error, request::ErrorKind);
    Types(types::Error, types::ErrorKind);
  }

  foreign_links {
    FromUtf8(string::FromUtf8Error);
    Http(http::Error);
    Hyper(hyper::Error);
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

  tokio::run(future::lazy(move || {
    let client = request::create_client_rc().unwrap();

    let app_1 = app.clone();
    let app_2 = app.clone();
    let client_1 = client.clone();
    let client_2 = client.clone();

    let args = std::env::args().collect::<Vec<_>>();

    let subreddit = args[1].clone();

    let limit: u32 = args[2].parse().unwrap();

    println!("authenticating...");

    auth::authenticate(app.clone(), client, tok, || "uwu")
      .from_err()
      .and_then(move |tok| {
        let tok_1 = tok.clone();
        let tok_2 = tok.clone();

        println!("saving apitok.json...");

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

            println!("listing r/{}...", subreddit);

            read::list_subreddit(
              app.clone(),
              tok.clone(),
              client.clone(),
              subreddit,
              // read::SortType::Top(read::SortRange::All),
              read::SortType::Hot,
              Some(limit), // TODO
            ).from_err()
          })
          .and_then(|listing| {
            let app = app_2;
            let tok = tok_2;
            let client = client_2;

            println!("retrieving comments...");

            fn print_comment(
              comment: &types::Comment,
              indent: &str,
              newline_re: &Regex,
            ) {
              if newline_re.is_match(&comment.body) {
                let body = format!("\n{}", comment.body);
                let body = newline_re
                  .replace_all(&body, format!("$1{}  : ", indent).as_str());

                println!(
                  "{}({:4}) u/{} ->{}",
                  indent, comment.score, comment.author, body,
                );
              } else {
                println!(
                  "{}({:4}) u/{:32}: {}",
                  indent, comment.score, comment.author, comment.body,
                );
              }

              match &comment.replies {
                types::CommentReplies::Some(l) => {
                  let l = l.clone().into_listing();
                  for reply in l.children {
                    match reply {
                      types::Thing::Comment(c) => {
                        print_comment(&c, &format!("{}  ", indent), &newline_re)
                      }
                      types::Thing::More(_) => println!("{}  ...", indent),
                      _ => println!("{}<unexpected Thing>", indent),
                    }
                  }
                }
                types::CommentReplies::None => {}
              }
            }

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
              .map(|v| {
                let newline_re = Regex::new(r"(\n+)").unwrap();

                for (links, comments) in v.iter() {
                  for link in links.clone().into_listing().children.iter() {
                    match link {
                      types::Thing::Link(l) => {
                        println!("## {} ({})", l.title, l.url);

                        if l.selftext.len() != 0 {
                          let body =
                            newline_re.replace_all(&l.selftext, "$1  # ");

                          println!("  # {}", body);
                        }
                      }
                      _ => println!("## <unexpected Thing>"),
                    }
                  }

                  for comment in comments.clone().into_listing().children.iter()
                  {
                    match comment {
                      types::Thing::Comment(c) => {
                        print_comment(&c, "  ", &newline_re)
                      }
                      types::Thing::More(_) => println!("  ..."),
                      _ => println!("  <unexpected Thing>"),
                    }
                  }

                  print!("\n\n\n");
                }
              })
          })
      })
      .map(|r| println!("return value: {:#?}", r))
      .map_err(|e: Error| println!("encountered an error: {}", e)) // TODO: make this less vague
  }));
}
