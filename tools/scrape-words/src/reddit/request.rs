use super::{client, RcAppInfo};
use futures::{future, prelude::*};
use http::{self, request};
use hyper;
use hyper_tls;
use serde;
use serde_json;
use std::{
  io::{self, Write},
  string,
  sync::{Arc, Mutex},
  thread,
  time::{Duration, Instant},
};
use thread_future::ThreadFuture;

error_chain! {
  foreign_links {
    FromUtf8(string::FromUtf8Error);
    Hyper(hyper::Error);
    HyperTls(hyper_tls::Error);
    SerdeJson(serde_json::Error);
  }

  errors {
    BadStatus(code: http::StatusCode, body: String) {
      description("got unexpected HTTP status"),
      display("got unexpected HTTP status {} (body: '{}')", code, body),
    }
  }
}

pub type Client = hyper::Client<
  hyper_tls::HttpsConnector<hyper::client::HttpConnector>,
  hyper::Body,
>;

pub type RcClient = Arc<Client>;

pub type Request = http::Request<hyper::Body>;

pub struct RatelimitData {
  remain: i32,
  reset: Instant,
  lock: Arc<Mutex<()>>,
}

pub type RcRatelimitData = Arc<Mutex<RatelimitData>>;

impl RatelimitData {
  pub fn new_rc() -> RcRatelimitData {
    Arc::new(Mutex::new(Default::default()))
  }
}

impl Default for RatelimitData {
  fn default() -> Self {
    RatelimitData {
      remain: -1,
      reset: Instant::now(),
      lock: Arc::new(Mutex::new(())),
    }
  }
}

pub fn create_client_rc() -> Result<RcClient> {
  let conn = hyper_tls::HttpsConnector::new(4)?;

  let client = hyper::Client::builder().build::<_, hyper::Body>(conn);

  Ok(Arc::new(client))
}

// NB: this CANNOT use client::RcClient as it's used in the authorization process,
//     before the client is actually created
pub fn create_request(app: RcAppInfo) -> request::Builder {
  let mut ret = http::Request::builder();

  ret.header("User-Agent", app.user_agent());

  ret
}

pub fn create_request_authorized(client: client::RcClient) -> request::Builder {
  let mut ret = create_request(client.app());

  ret.header(
    "Authorization",
    format!("bearer {}", client.tok().access_token()),
  );

  ret
}

// TODO: don't forget to deal with token refreshing
pub fn request_string(
  client: RcClient,
  rl: RcRatelimitData,
  req: Request,
) -> impl Future<Item = String, Error = Error> {
  let rl_1 = rl.clone();
  let rl_2 = rl.clone();

  let uri_1 = req.uri().clone();

  ThreadFuture::new(move || {
    let now = Instant::now();

    let sleep_time;
    let lock;

    {
      let mut rl = rl_1.lock().unwrap();

      lock = rl.lock.clone();

      sleep_time = if now > rl.reset || rl.remain < 0 {
        Duration::from_secs(0)
      } else {
        let remain = rl.remain as u64;
        let reset = (rl.reset - now).as_secs();

        if remain < reset {
          Duration::from_secs(reset - remain)
        } else {
          Duration::from_secs(0)
        }
      };

      if rl.remain > 0 {
        rl.remain = rl.remain - 1;
      }
    }

    // TODO: this lock should probably extend further upwards in this code
    {
      let _ = lock.lock().unwrap();

      if sleep_time > Duration::from_secs(0) {
        writeln!(
          io::stderr(),
          "halting for {}s + {}ms",
          sleep_time.as_secs(),
          sleep_time.subsec_millis()
        ).unwrap();
        thread::sleep(sleep_time);
      }
    }

    Ok(())
  }).and_then(move |_| {
    client
      .request(req)
      .from_err()
      .and_then(move |res| {
        let status = res.status();

        // TODO: keep an eye on out-of-sync responses

        // if res.headers().contains_key("x-ratelimit-remaining") {
        //   write!(
        //     io::stderr(),
        //     "x-ratelimit-remaining: {}\n",
        //     match res.headers()["x-ratelimit-remaining"].to_str() {
        //       Ok(s) => s,
        //       Err(_) => "<non-text>",
        //     }
        //   ).unwrap();
        // }

        // if res.headers().contains_key("x-ratelimit-reset") {
        //   write!(
        //     io::stderr(),
        //     "x-ratelimit-reset: {}\n",
        //     match res.headers()["x-ratelimit-reset"].to_str() {
        //       Ok(s) => s,
        //       Err(_) => "<non-text>",
        //     }
        //   ).unwrap();
        // }

        {
          let mut rl = rl_2.lock().unwrap();

          if res.headers().contains_key("x-ratelimit-remaining") {
            rl.remain = res.headers()["x-ratelimit-remaining"]
              .to_str()
              .unwrap()
              .parse::<f64>()
              .unwrap() as i32;
          }

          if res.headers().contains_key("x-ratelimit-reset") {
            rl.reset = Instant::now()
              + Duration::from_secs(
                res.headers()["x-ratelimit-reset"]
                  .to_str()
                  .unwrap()
                  .parse::<f64>()
                  .unwrap() as u64,
              );
          }
        }

        res
          .into_body()
          .collect()
          .from_err()
          .map(move |v| (v, status))
      })
      .and_then(|(vec, status)| {
        let bytes: Vec<u8> = vec.iter().flat_map(|c| c.to_vec()).collect();

        String::from_utf8(bytes)
          .into_future()
          .from_err()
          .map(move |s| (s, status))
      })
      .and_then(|(string, status)| match status {
        http::StatusCode::OK => future::ok(string),
        s => future::err(ErrorKind::BadStatus(s, string).into()),
      })
  })
}

// TODO: better error handling
pub fn request_json<T>(
  client: RcClient,
  rl: RcRatelimitData,
  req: Request,
) -> impl Future<Item = T, Error = Error>
where
  T: for<'de> serde::Deserialize<'de>,
{
  request_string(client, rl, req).and_then(|string| {
    // println!("string: {}", string);
    serde_json::from_str(&string).into_future().from_err()
  })
}
