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

use futures::{Future, IntoFuture};
use reddit::auth::{self, AppDuration, AppId, AppInfo, AuthToken};
use std::{
  fs::File,
  io::{self, BufReader, BufWriter},
};

error_chain! {
  links {
    Auth(auth::Error, auth::ErrorKind);
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

    serde_json::from_reader(file).map_err(|e| e.into()) // TODO: is there a cleaner way to do this?
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
    "read",
  );

  tokio::run(
    auth::authenticate(app.clone(), tok, &|| "uwu")
      .map_err(|e| e.into())
      .and_then(|tok| {
        File::create("etc/apitok.json")
          .into_future()
          .map(|f| (f, tok))
          .map_err(|e| e.into())
      })
      .and_then(|(file, tok)| {
        let file = BufWriter::new(file);

        serde_json::to_writer(file, &tok)
          .into_future()
          .map_err(|e| e.into())
      })
      .map_err(|e: Error| println!("encountered an error: {}", e)), // TODO: make this less vague
  );
}
