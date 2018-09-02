use super::{auth::RcAuthToken, RcAppInfo};
use futures::{future, Future, IntoFuture, Stream};
use http::{self, request};
use hyper;
use hyper_tls;
use serde;
use serde_json;
use std::{string, sync::Arc};

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

pub fn create_client_rc() -> Result<RcClient> {
  let conn = hyper_tls::HttpsConnector::new(4)?;

  let client = hyper::Client::builder().build::<_, hyper::Body>(conn);

  Ok(Arc::new(client))
}

pub fn create_request(app: RcAppInfo) -> request::Builder {
  let mut ret = http::Request::builder();

  ret.header("User-Agent", app.user_agent());

  ret
}

pub fn create_request_authorized(
  app: RcAppInfo,
  tok: RcAuthToken,
) -> request::Builder {
  let mut ret = create_request(app);

  ret.header("Authorization", format!("bearer {}", tok.access_token()));

  ret
}

pub fn request_string(
  client: RcClient,
  req: Request,
) -> impl Future<Item = String, Error = Error> {
  client
    .request(req)
    .from_err()
    .and_then(|res| {
      let status = res.status();

      println!("headers: {:#?}", res.headers());

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
}

pub fn request_json<T>(
  client: RcClient,
  req: Request,
) -> impl Future<Item = T, Error = Error>
where
  T: for<'de> serde::Deserialize<'de>,
{
  request_string(client, req)
    .and_then(|string| serde_json::from_str(&string).into_future().from_err())
}
