use base64;
use futures::{
  future::{self, Future},
  stream::Stream,
  IntoFuture,
};
use http::{header, Uri};
use hyper;
use hyper_tls;
use serde_json;
use std::{
  io::{self, Write},
  sync::Arc,
};
use url::form_urlencoded;

#[derive(Serialize, Deserialize)]
pub struct AppId {
  id: String,
  secret: String,
}

// impl AppId {
//   pub fn id(&self) -> &str {
//     &self.id
//   }
// }

pub enum AppDuration {
  Temporary,
  Permanent,
}

impl ToString for AppDuration {
  fn to_string(&self) -> String {
    match self {
      AppDuration::Temporary => "temporary",
      AppDuration::Permanent => "permanent",
    }.to_string()
  }
}

pub struct AppInfo {
  id: AppId,
  redir: Uri,
  duration: AppDuration,
  scope: String, // TODO: make this not a string maybe?
}

type RcAppInfo = Arc<AppInfo>;

impl AppInfo {
  pub fn new_rc(
    id: AppId,
    redir: Uri,
    duration: AppDuration,
    scope: &str,
  ) -> RcAppInfo {
    Arc::new(Self {
      id,
      redir,
      duration,
      scope: scope.to_string(),
    })
  }
}

#[derive(Debug, Deserialize)]
struct AuthTokenResponse {
  error: Option<String>,
  access_token: Option<String>,
  token_type: Option<String>,
  expires_in: Option<u32>,
  scope: Option<String>,
  refresh_token: Option<String>,
}

#[derive(Debug)]
pub struct AuthToken {
  access_token: String,
  token_type: String,
  expires_in: u32,
  scope: String, // TODO: is this necessary?
  refresh_token: String,
}

impl Into<AuthToken> for AuthTokenResponse {
  fn into(self) -> AuthToken {
    AuthToken {
      access_token: self.access_token.unwrap(),
      token_type: self.token_type.unwrap(),
      expires_in: self.expires_in.unwrap(),
      scope: self.scope.unwrap(),
      refresh_token: self.refresh_token.unwrap(),
    }
  }
}

fn authcode_uri<'state, StateFn>(app: RcAppInfo, state: &StateFn) -> Uri
where
  StateFn: Fn() -> &'state str,
{
  let query = form_urlencoded::Serializer::new(String::new())
    .extend_pairs(
      [
        ("client_id", app.id.id.clone()),
        ("response_type", "code".to_string()),
        ("state", state().to_string()),
        ("redirect_uri", app.redir.to_string()),
        ("duration", app.duration.to_string()),
        ("scope", app.scope.clone()),
      ].iter(),
    )
    .finish();

  format!("https://www.reddit.com/api/v1/authorize?{}", query)
    .parse()
    .unwrap()
}

fn get_authcode<'auth_state, AuthStateFn>(
  app: RcAppInfo,
  auth_state: &AuthStateFn,
) -> Result<(String, usize), io::Error>
where
  AuthStateFn: Fn() -> &'auth_state str,
{
  let mut code = String::new();

  writeln!(
    io::stderr(),
    "authcode required - get it from {}",
    authcode_uri(app, auth_state)
  )?;

  write!(io::stderr(), "paste your code here: ")?;
  io::stderr().flush()?;

  let n = io::stdin().read_line(&mut code)?;

  code = code.trim().to_string();

  if n == 0 {
    write!(io::stderr(), "\n")?;
  }

  Ok((code, n))
}

fn auth_token_body(app: RcAppInfo, code: &str) -> String {
  form_urlencoded::Serializer::new(String::new())
    .extend_pairs(
      [
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", app.redir.to_string()),
      ].iter(),
    )
    .finish()
}

pub fn authenticate_with_code<'auth_state, AuthStateFn>(
  app: RcAppInfo,
  auth_state: &AuthStateFn,
) -> impl Future<Item = AuthToken, Error = ()>
where
  AuthStateFn: Fn() -> &'auth_state str,
{
  let conn = hyper_tls::HttpsConnector::new(4).unwrap(); // TODO
  let client = Arc::new(hyper::Client::builder().build::<_, hyper::Body>(conn));

  // I hate this.
  let app_1 = app.clone();
  let client_1 = client.clone();

  get_authcode(app, auth_state)
    .into_future()
    .map_err(|_| ()) // TODO
    .and_then(|(code, n)| if n == 0 {
      future::err(()) // TODO
    } else {
      future::ok(code)
    })
    .and_then(move |code| {
      let app = app_1;

      println!("auth code: {}", code);

      let mut builder = super::create_request();

      let uri: Uri = "https://www.reddit.com/api/v1/access_token"
        .parse()
        .unwrap();

      builder.method("POST").uri(uri).header(
        header::AUTHORIZATION,
        format!(
          "Basic {}",
          base64::encode(&format!("{}:{}", app.id.id, app.id.secret))
        ),
      );

      let req_body = hyper::Body::from(auth_token_body(app.clone(), &code));

      builder.body(req_body).into_future().map_err(|_| ()) // TODO
    })
    .and_then(move |req| {
      let client = client_1;

      client.request(req).map_err(|_| ()) // TODO
    })
    .and_then(|res| {
      println!("response: {:#?}", res);

      // TODO: check status, etc.

      res.into_body().collect().map_err(|_| ()) // TODO
    })
    .and_then(|vec| {
      // TODO: this seems mildly problematic
      let bytes: Vec<u8> = vec.iter().flat_map(|c| c.to_vec()).collect();

      String::from_utf8(bytes).into_future().map_err(|_| ()) // TODO
    })
    .and_then(|string| {
      // TODO: why did rustfmt stop working?
      let ret: Result<AuthTokenResponse, serde_json::Error> = serde_json::from_str(&string);

      ret.map_err(|_| ()) // TODO
    })
    .and_then(|val| {
      println!("body: {:#?}", val);

      match val.error {
        Some(e) => future::err(e), // TODO: make this better
        None => future::ok(val),
      }.map_err(|_| ()) // TODO
    })
    .map(|val| val.into()) // TODO: ...wait, why is this a todo?
}

pub fn authenticate<'auth_state, AuthStateFn>(
  app: RcAppInfo,
  auth_state: &AuthStateFn,
) -> impl Future<Item = AuthToken, Error = ()>
where
  AuthStateFn: Fn() -> &'auth_state str,
{
  authenticate_with_code(app, auth_state)
}
