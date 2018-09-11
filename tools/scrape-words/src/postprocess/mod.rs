use std::str::FromStr;
use {ParseError, ParseErrorKind, ParseResult};

mod process;
mod recover;

pub use self::process::*;

pub enum Proc {
  Analyze,
  Dump,
}

impl FromStr for Proc {
  type Err = ParseError;

  fn from_str(s: &str) -> ParseResult<Self> {
    match s {
      "analyze" => Ok(Proc::Analyze),
      "dump" => Ok(Proc::Dump),
      s => Err(ParseErrorKind::NoMatch(s.into()).into()),
    }
  }
}
