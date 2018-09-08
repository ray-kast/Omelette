use super::{app_info, auth, request};
use futures::{Future, IntoFuture};
use std::sync::Arc;

error_chain!{
  links {
    Auth(auth::Error, auth::ErrorKind);
    Request(request::Error, request::ErrorKind);
  }

  errors {
    ClientHookError(hook: String) {
      description("client hook failed"),
      display("client hook {} failed", hook),
    }
  }
}

// TODO: figure out if I actually need all this refcounting

pub trait ClientHooks {
  fn auth_code_state(&self) -> String;

  fn save_token(
    &self,
    tok: auth::RcAuthToken,
  ) -> Box<dyn Future<Item = (), Error = ()> + Send>;
}

pub type ClientHookObject = Arc<dyn ClientHooks + Send + Sync>;

// #[derive(Debug)] // TODO: fix hooks
pub struct Client {
  hooks: ClientHookObject, // TODO: Mutex this?
  client: request::RcClient,
  rl: request::RcRatelimitData,
  app: app_info::RcAppInfo,
  tok: auth::RcAuthToken,
}

pub type RcClient = Arc<Client>;

impl Client {
  pub fn new(
    hooks: ClientHookObject,
    app: app_info::RcAppInfo,
    tok: Option<auth::AuthToken>,
    concurrency: usize,
  ) -> impl Future<Item = RcClient, Error = Error> {
    request::create_client_rc()
      .into_future()
      .from_err()
      .and_then(move |client| {
        let hooks_1 = hooks.clone();

        let rl = request::RatelimitData::new_rc(concurrency);

        auth::authenticate(
          app.clone(),
          client.clone(),
          rl.clone(),
          tok,
          move || hooks_1.auth_code_state(),
        ).from_err()
          .and_then(|tok| {
            hooks
              .save_token(tok.clone())
              .map_err(|_| {
                ErrorKind::ClientHookError("save_token".into()).into()
              })
              .map(|_| {
                Arc::new(Self {
                  hooks,
                  client,
                  rl,
                  app,
                  tok,
                })
              })
          })
      })
  }

  pub fn client(&self) -> request::RcClient {
    self.client.clone()
  }

  pub fn rl(&self) -> request::RcRatelimitData {
    self.rl.clone()
  }

  pub fn app(&self) -> app_info::RcAppInfo {
    self.app.clone()
  }

  pub fn tok(&self) -> auth::RcAuthToken {
    self.tok.clone()
  }
}
