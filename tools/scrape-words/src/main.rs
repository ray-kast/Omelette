extern crate base64;
extern crate error_chain;
extern crate futures;
extern crate http;
extern crate hyper;
extern crate hyper_tls;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate url;

#[macro_use]
extern crate serde_derive;

mod reddit;

use futures::Future;
use reddit::{AppDuration, AppId, AppInfo};
use std::{fs::File, io::BufReader};

fn main() {
  let id: AppId;

  {
    let file =
      File::open("etc/apikey.json").expect("couldn't open apikey.json");
    let file = BufReader::new(file);

    id = serde_json::from_reader(file).expect("failed to parse apikey.json");
  }

  let app = AppInfo::new_rc(
    id,
    "http://rk1024.net/".parse().unwrap(),
    AppDuration::Permanent,
    "read",
  );

  tokio::run(
    reddit::authenticate(app.clone(), &|| "uwu")
      .map(|tok| {
        println!("token: {:#?}", tok);
      })
      .map_err(|_| println!("authentication failed.")), // TODO
  );
}
