use super::{client, RcAppInfo};
use future_semaphore::FutureSemaphore;
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
    Unit {
      description("unknown unit error"),
      display("unknown unit error"),
    }

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
  remain: Option<i32>,
  reset: Instant,
  last_update: Instant,
  lock: Arc<Mutex<()>>,
  sem: FutureSemaphore,
}

pub type RcRatelimitData = Arc<Mutex<RatelimitData>>;

impl RatelimitData {
  pub fn new_rc(concurrency: usize) -> RcRatelimitData {
    Arc::new(Mutex::new(RatelimitData {
      remain: None,
      reset: Instant::now(),
      last_update: Instant::now(),
      lock: Arc::new(Mutex::new(())),
      sem: FutureSemaphore::new(concurrency),
    }))
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

  let rl = rl.lock().unwrap();

  rl.sem
    .enter()
    .map_err(|_| ErrorKind::Unit.into())
    .and_then(move |guard| {
      ThreadFuture::new(move || {
        let sleep_time;
        let lock = {
          let rl = rl_1.lock().unwrap();
          rl.lock.clone()
        };
        let _lock = lock.lock().unwrap();

        {
          let mut rl = rl_1.lock().unwrap();
          let now = Instant::now();

          sleep_time = match rl.remain {
            None => Duration::from_secs(0),
            Some(remain) => {
              rl.remain = Some(remain - 1);

              let remain = remain as i64;
              let reset = (rl.reset - now).as_secs() as i64;

              if remain < 0 {
                Duration::from_secs(10)
              } else {
                if remain < reset {
                  Duration::from_secs((reset - remain) as u64)
                } else {
                  Duration::from_secs(0)
                }
              }
            }
          };
          rl.last_update = now;
        }

        if sleep_time > Duration::from_secs(0) {
          if sleep_time > Duration::from_secs(10) {
            writeln!(
              io::stderr(),
              "\nhalting for {}s (remain: {})",
              sleep_time.as_secs() as f64
                + sleep_time.subsec_millis() as f64 / 1000.0,
              rl_1.lock().unwrap().remain.unwrap(),
            ).unwrap();
          }

          thread::sleep(sleep_time);
        }

        Ok(())
      }).and_then(move |_| {
        client
          .request(req)
          .from_err()
          .and_then(move |res| {
            let status = res.status();

            {
              let mut rl = rl_2.lock().unwrap();

              let now = Instant::now();
              let do_update = rl.remain.is_none() || rl.last_update < now;

              if res.headers().contains_key("x-ratelimit-remaining") {
                let remain = res.headers()["x-ratelimit-remaining"]
                  .to_str()
                  .unwrap()
                  .parse::<f64>()
                  .unwrap() as i32;

                if do_update {
                  rl.remain = Some(remain);
                }

                // if remain < 20 {
                //   writeln!(io::stderr(), "\nx-ratelimit-remain: {}", remain)
                //     .unwrap();
                // }
              }

              if res.headers().contains_key("x-ratelimit-reset") {
                if do_update {
                  rl.reset = now
                    + Duration::from_secs(
                      res.headers()["x-ratelimit-reset"]
                        .to_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap(),
                    );
                }
              }

              if do_update {
                rl.last_update = now;
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
          .and_then(|(string, status)| {
            let _guard = guard;
            match status {
              http::StatusCode::OK => future::ok(string),
              s => future::err(ErrorKind::BadStatus(s, string).into()),
            }
          })
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
