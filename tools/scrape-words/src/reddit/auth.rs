use super::{
  request::{self, RcClient},
  AppDuration, RcAppInfo,
};
use base64;
use futures::{
  future::{self, Either, Future},
  IntoFuture,
};
use http::{self, header, Uri};
use hyper;
use std::{
  collections::BTreeSet,
  io::{self, Write},
  sync::Arc,
};
use url::form_urlencoded;

error_chain! {
  links {
    Request(request::Error, request::ErrorKind);
  }

  foreign_links {
    Http(http::Error);
    Io(io::Error);
  }

  errors {
    ApiError(msg: String) {
      description("Reddit API error"),
      display("Reddit API error '{}'", msg),
    }

    MissingRefreshTok {
      description("missing refresh token"),
      display("missing refresh token (probably a temporary app)"),
    }

    Eof(at: String) {
      description("stdin closed"),
      display("stdin closed while reading {}", at),
    }

    InvalidGrant {
      description("invalid grant"),
      display("invalid grant (probably expired authcode)"),
    }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
  access_token: String,
  token_type: String,
  expires_in: u32,
  scope: BTreeSet<String>,
  refresh_token: Option<String>,
}

impl AuthToken {
  pub fn access_token(&self) -> &str {
    &self.access_token
  }
}

pub type RcAuthToken = Arc<AuthToken>;

impl Into<AuthToken> for AuthTokenResponse {
  fn into(self) -> AuthToken {
    AuthToken {
      access_token: self.access_token.unwrap(),
      token_type: self.token_type.unwrap(),
      expires_in: self.expires_in.unwrap(),
      scope: self
        .scope
        .unwrap()
        .split(" ")
        .map(|e| e.to_string())
        .collect(),
      refresh_token: self.refresh_token,
    }
  }
}

fn authcode_uri<'state, StateFn>(app: RcAppInfo, state: StateFn) -> Uri
where
  StateFn: Fn() -> &'state str,
{
  let query = form_urlencoded::Serializer::new(String::new())
    .extend_pairs(
      [
        ("client_id", app.id().id().to_string()),
        ("response_type", "code".to_string()),
        ("state", state().to_string()),
        ("redirect_uri", app.redir().to_string()),
        ("duration", app.duration().to_string()),
        (
          "scope",
          app
            .scope()
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join(" "), // TODO: use the iterator join method when it's available
        ),
      ].iter(),
    )
    .finish();

  format!("https://www.reddit.com/api/v1/authorize?{}", query)
    .parse()
    .unwrap()
}

fn get_authcode<'auth_state, AuthStateFn>(
  app: RcAppInfo,
  auth_state: AuthStateFn,
) -> Result<(String, usize)>
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

fn create_access_token_request(app: &RcAppInfo) -> http::request::Builder {
  let mut builder = request::create_request(app.clone());

  builder
    .method("POST")
    .uri("https://www.reddit.com/api/v1/access_token")
    .header(
      header::AUTHORIZATION,
      format!(
        "basic {}",
        base64::encode(&format!("{}:{}", app.id().id(), app.id().secret()))
      ),
    );

  builder
}

fn auth_token_body_for_code(app: RcAppInfo, code: &str) -> String {
  form_urlencoded::Serializer::new(String::new())
    .extend_pairs(
      [
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", app.redir().to_string()),
      ].iter(),
    )
    .finish()
}

fn auth_token_body_for_refresh(tok: &str) -> String {
  form_urlencoded::Serializer::new(String::new())
    .extend_pairs(
      [
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", tok.to_string()),
      ].iter(),
    )
    .finish()
}

pub fn authenticate_with_code<'auth_state, AuthStateFn>(
  app: RcAppInfo,
  client: RcClient,
  auth_state: AuthStateFn,
) -> impl Future<Item = RcAuthToken, Error = Error>
where
  AuthStateFn: Fn() -> &'auth_state str,
{
  let app_1 = app.clone();
  let client_1 = client.clone();

  get_authcode(app, auth_state)
    .into_future()
    .and_then(|(code, n)| {
      if n == 0 {
        future::err(ErrorKind::Eof("authcode".into()).into())
      } else {
        future::ok(code)
      }
    })
    .and_then(|code| {
      let app = app_1;

      create_access_token_request(&app)
        .body(hyper::Body::from(auth_token_body_for_code(
          app.clone(),
          &code,
        )))
        .into_future()
        .from_err()
        .and_then(|req| {
          let client = client_1;

          request::request_json(client, req).from_err()
        })
        .and_then(|val: AuthTokenResponse| match val.error {
          Some(e) => if e == "invalid_grant" {
            future::err(ErrorKind::InvalidGrant.into())
          } else {
            future::err(ErrorKind::ApiError(e).into())
          },
          None => future::ok(Arc::new(val.into())),
        })
    })
}

pub fn authenticate_with_refresh(
  app: RcAppInfo,
  client: RcClient,
  tok: AuthToken,
) -> impl Future<Item = RcAuthToken, Error = Error> {
  let app_1 = app.clone();
  let client_1 = client.clone();

  match tok.refresh_token {
    Some(s) => future::ok(s),
    None => future::err(ErrorKind::MissingRefreshTok.into()),
  }.and_then(|s| {
    let app = app_1;

    create_access_token_request(&app)
      .body(hyper::Body::from(auth_token_body_for_refresh(&s)))
      .into_future()
      .from_err()
      .and_then(|req| {
        let client = client_1;

        request::request_json(client, req).from_err()
      })
      .and_then(|val: AuthTokenResponse| match val.error {
        Some(e) => future::err(ErrorKind::ApiError(e.into()).into()),
        None => future::ok(Arc::new(AuthToken {
          refresh_token: Some(s),
          ..val.into()
        })),
      })
  })
}

pub fn authenticate<'auth_state, AuthStateFn>(
  app: RcAppInfo,
  client: RcClient,
  tok: Option<AuthToken>,
  auth_state: AuthStateFn,
) -> impl Future<Item = RcAuthToken, Error = Error>
where
  AuthStateFn: Fn() -> &'auth_state str,
{
  match tok {
    Some(tok) => {
      let app_1 = app.clone();
      let client_1 = client.clone();

      if &tok.scope == app.scope() && match tok.refresh_token {
        Some(_) => AppDuration::Permanent,
        None => AppDuration::Temporary,
      } == app.duration()
      {
        Either::A(authenticate_with_refresh(app, client, tok).or_else(|err| {
          println!("failed to refresh token: {}", err);

          authenticate_with_code(app_1, client_1, auth_state)
        }))
      } else {
        println!("token data mismatch, requesting new token");

        Either::B(authenticate_with_code(app, client, auth_state))
      }
    }
    None => Either::B(authenticate_with_code(app, client, auth_state)),
  }
}
