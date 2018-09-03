use super::{
  auth::RcAuthToken,
  request::{self, RcClient},
  types, RcAppInfo,
};
use futures::{Future, IntoFuture};
use http;
use hyper;
use url::{form_urlencoded, percent_encoding};

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

// TODO: add the API parameters here
pub fn list_subreddit(
  app: RcAppInfo,
  tok: RcAuthToken,
  client: RcClient,
  subreddit: String,
  sort: SortType,
  limit: Option<u32>, // TODO: do the thing with From<>
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
    Some(c) => {
      query.append_pair("limit", &c.to_string());
    }
    None => {}
  }

  let query = query.finish();

  request::create_request_authorized(app.clone(), tok.clone())
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
    .and_then(|req| request::request_json(client, req).from_err())
    .and_then(|thing: types::Thing| {
      thing.try_into_listing().into_future().from_err()
    })
}

pub fn get_comments(
  app: RcAppInfo,
  tok: RcAuthToken,
  client: RcClient,
  link: &types::Link,
) -> impl Future<Item = (types::Thing, types::Thing), Error = Error> {
  // TODO: what's the actual type of the return?

  request::create_request_authorized(app.clone(), tok.clone())
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
    .and_then(|req| request::request_json(client, req).from_err())
}
