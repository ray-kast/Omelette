extern crate base64;
extern crate futures;
extern crate http;
extern crate hyper;
extern crate hyper_tls;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate url;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

mod reddit;

use futures::{future, Future, IntoFuture};
use reddit::{
  auth::{self, AuthToken},
  read, request, AppDuration, AppId, AppInfo,
};
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

  tokio::run(future::lazy(move || {
    let client = request::create_client_rc().unwrap();

    let app_1 = app.clone();
    let client_1 = client.clone();

    auth::authenticate(app.clone(), client, tok, &|| "uwu")
      .from_err()
      .and_then(|tok| {
        let tok_1 = tok.clone();

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
          .and_then(|_| {
            let app = app_1;
            let tok = tok_1;
            let client = client_1;

            read::list_subreddit(
              app.clone(),
              tok.clone(),
              client.clone(),
              "nice_guys".into(),
              read::SortType::Top(read::SortRange::All),
            ).from_err()
          })
      })
      .map(|_| ())
      // .map(|r| println!("return value: {:#?}", r))
      .map_err(|e: Error| println!("encountered an error: {}", e)) // TODO: make this less vague
  }));
}
