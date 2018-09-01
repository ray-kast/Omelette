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

use futures::Future;
use reddit::auth::{self, AppDuration, AppId, AppInfo, AuthToken};
use std::{fs::File, io::BufReader};

fn main() {
  let id: AppId;

  {
    let file =
      File::open("etc/apikey.json").expect("couldn't open apikey.json");
    let file = BufReader::new(file);

    id = serde_json::from_reader(file).expect("failed to parse apikey.json");
  }

  fn retrieve_token() -> Result<AuthToken, ()> {
    let file = File::open("etc/apitoken.json").map_err(|_| ())?; // TODO
    let file = BufReader::new(file);

    serde_json::from_reader(file).map_err(|_| ())? // TODO
  }

  let tok = match retrieve_token() {
    Ok(tok) => Some(tok),
    Err(_) => {
      // TODO
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
      .map(|tok| {
        println!("token: {:#?}", tok);
      })
      .map_err(|e| println!("authentication failed: {}", e)),
  );
}
