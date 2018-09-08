use std::str::FromStr;
use {ParseError, ParseErrorKind, ParseResult};

pub mod local;
pub mod reddit;

pub enum Source {
  Reddit,
  Local,
}

impl FromStr for Source {
  type Err = ParseError;

  fn from_str(s: &str) -> ParseResult<Self> {
    match s {
      "reddit" => Ok(Source::Reddit),
      "local" => Ok(Source::Local),
      s => Err(ParseErrorKind::NoMatch(s.into()).into()),
    }
  }
}
