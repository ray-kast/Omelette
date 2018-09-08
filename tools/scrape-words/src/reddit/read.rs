use super::{client::RcClient, request, types};
use futures::{Future, IntoFuture};
use http;
use hyper;
use regex::Regex;
use std::str::FromStr;
use url::{form_urlencoded, percent_encoding};
use {ParseError, ParseErrorKind, ParseResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortRange {
  Hour,
  Day,
  Week,
  Month,
  Year,
  All,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortType {
  Hot,
  New,
  Rising,
  Top(SortRange),
  Controversial(SortRange),
}

error_chain!{
  links {
    Request(request::Error, request::ErrorKind);
    Types(types::Error, types::ErrorKind);
  }

  foreign_links {
    Http(http::Error);
  }
}

impl ToString for SortRange {
  fn to_string(&self) -> String {
    match self {
      SortRange::Hour => "hour",
      SortRange::Day => "day",
      SortRange::Week => "week",
      SortRange::Month => "month",
      SortRange::Year => "year",
      SortRange::All => "all",
    }.to_string()
  }
}

impl FromStr for SortRange {
  type Err = ParseError;

  fn from_str(s: &str) -> ParseResult<Self> {
    match s {
      "hour" => Ok(SortRange::Hour),
      "day" => Ok(SortRange::Day),
      "week" => Ok(SortRange::Week),
      "month" => Ok(SortRange::Month),
      "year" => Ok(SortRange::Year),
      "all" => Ok(SortRange::All),
      s => Err(ParseErrorKind::NoMatch(s.into()).into()),
    }
  }
}

// TODO: this is not symmetric with FromString
impl ToString for SortType {
  fn to_string(&self) -> String {
    match self {
      SortType::Hot => "hot",
      SortType::New => "new",
      SortType::Rising => "rising",
      SortType::Top(_) => "top",
      SortType::Controversial(_) => "controversial",
    }.to_string()
  }
}

impl FromStr for SortType {
  type Err = ParseError;

  fn from_str(s: &str) -> ParseResult<Self> {
    lazy_static! {
      static ref RANGED_RE: Regex = Regex::new(r"(\w+)\s*\((\w+)\)").unwrap();
    }

    match s {
      "hot" => Ok(SortType::Hot),
      "new" => Ok(SortType::New),
      "rising" => Ok(SortType::Rising),
      s => match RANGED_RE.captures(s) {
        Some(c) => {
          let range = match c.get(2).map(|m| m.as_str()) {
            Some(s) => s.parse(),
            None => Err(
              ParseErrorKind::BadSyntax(
                s.into(),
                "<type> or <type>(<range>)".into(),
              ).into(),
            ),
          }?;

          match c.get(1).map(|m| m.as_str()) {
            Some("top") => Ok(SortType::Top(range)),
            Some("controversial") => Ok(SortType::Controversial(range)),
            Some(s) => Err(ParseErrorKind::NoMatch(s.into()).into()),
            None => Err(
              ParseErrorKind::BadSyntax(
                s.into(),
                "<type> or <type>(<range>)".into(),
              ).into(),
            ),
          }
        }
        None => Err(ParseErrorKind::NoMatch(s.into()).into()),
      },
    }
  }
}

// TODO: add the API parameters here
pub fn list_subreddit(
  client: RcClient,
  subreddit: String,
  sort: SortType,
  limit: Option<u32>, // TODO: do the thing with From<>
  after: Option<String>,
) -> impl Future<Item = types::Listing, Error = Error> {
  let mut query = form_urlencoded::Serializer::new(String::new());

  match sort {
    SortType::Top(range) => {
      query.append_pair("t", &range.to_string());
    }
    SortType::Controversial(range) => {
      query.append_pair("t", &range.to_string());
    }
    _ => {}
  }

  match limit {
    Some(l) => {
      query.append_pair("limit", &l.to_string());
    }
    None => {}
  }

  match after {
    Some(a) => {
      query.append_pair("after", &a);
    }
    None => {}
  }

  let query = query.finish();

  request::create_request_authorized(client.clone())
    .method("GET")
    .uri(format!(
      "https://oauth.reddit.com/r/{}/{}?{}",
      percent_encoding::utf8_percent_encode(
        &subreddit,
        percent_encoding::PATH_SEGMENT_ENCODE_SET
      ),
      sort.to_string(),
      query
    ))
    .body(hyper::Body::empty())
    .into_future()
    .from_err()
    .and_then(move |req| {
      request::request_json(client.client(), client.rl(), req).from_err()
    })
    .and_then(|thing: types::Thing| {
      thing.try_into_listing().into_future().from_err()
    })
}

pub fn get_comments(
  client: RcClient,
  link: &types::Link,
) -> impl Future<Item = (types::Thing, types::Thing), Error = Error> {
  // TODO: what's the actual type of the return?

  request::create_request_authorized(client.clone())
    .method("GET")
    .uri(format!(
      "https://oauth.reddit.com/comments/{}",
      percent_encoding::utf8_percent_encode(
        &link.id,
        percent_encoding::PATH_SEGMENT_ENCODE_SET
      )
    ))
    .body(hyper::Body::empty())
    .into_future()
    .from_err()
    .and_then(move |req| {
      request::request_json(client.client(), client.rl(), req).from_err()
    })
}
