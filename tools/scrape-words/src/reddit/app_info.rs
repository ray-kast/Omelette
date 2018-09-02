use http::Uri;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Serialize, Deserialize)]
pub struct AppId {
  id: String,
  secret: String,
}

impl AppId {
  pub fn id(&self) -> &str {
    &self.id
  }

  pub fn secret(&self) -> &str {
    &self.secret
  }
}

#[derive(Clone, Copy, PartialEq)]
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

// TODO: this probably belongs in another module
pub struct AppInfo {
  id: AppId,
  redir: Uri,
  duration: AppDuration,
  scope: BTreeSet<String>, // TODO: make this not a string maybe?
  platform: String,
  friendly_id: String,
  version: String,
  author_handle: String,
}

pub type RcAppInfo = Arc<AppInfo>;

impl AppInfo {
  pub fn new_rc<'a, I>(
    id: AppId,
    redir: Uri,
    duration: AppDuration,
    scope: I,
    platform: &str,
    friendly_id: &str,
    version: &str,
    author_handle: &str,
  ) -> RcAppInfo
  where
    I: Iterator<Item = &'a str>,
  {
    Arc::new(Self {
      id,
      redir,
      duration,
      scope: scope.map(|e| e.to_string()).collect(),
      platform: platform.to_string(),
      friendly_id: friendly_id.to_string(),
      version: version.to_string(),
      author_handle: author_handle.to_string(),
    })
  }

  pub fn user_agent(&self) -> String {
    format!(
      "{}:{}:{} (by /u/{})",
      self.platform, self.friendly_id, self.version, self.author_handle
    )
  }

  pub fn id(&self) -> &super::AppId {
    &self.id
  }

  pub fn redir(&self) -> &Uri {
    &self.redir
  }

  pub fn duration(&self) -> AppDuration {
    self.duration
  }

  pub fn scope(&self) -> &BTreeSet<String> {
    &self.scope
  }
}
