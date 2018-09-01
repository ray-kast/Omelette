use http::{request, Request};

pub fn create_request() -> request::Builder {
  let mut ret = Request::builder();

  ret.header("User-Agent", "rk1024/scrape-words");

  ret
}
