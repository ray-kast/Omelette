#![recursion_limit = "128"]

extern crate base64;
extern crate futures;
extern crate html5ever;
extern crate http;
extern crate hyper;
extern crate hyper_tls;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate serde_value;
extern crate tokio;
extern crate unicode_normalization;
extern crate url;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

mod future_semaphore;
mod postprocess;
mod process_html;
mod reddit;
mod scrape;
mod thread_future;

use futures::prelude::*;
use reddit::{auth, client, read, request, types, AppDuration, AppInfo};
use std::{
  collections::VecDeque,
  env,
  fs::File,
  io::{self, prelude::*},
  string,
};

// TODO: I should probably consolidate reddit::*::Error
error_chain! {
  links {
    Auth(auth::Error, auth::ErrorKind);
    Client(client::Error, client::ErrorKind);
    Parse(ParseError, ParseErrorKind);
    ProcessHtml(process_html::Error, process_html::ErrorKind);
    Read(read::Error, read::ErrorKind);
    Request(request::Error, request::ErrorKind);
    Types(types::Error, types::ErrorKind);
  }

  foreign_links {
    FromUtf8(string::FromUtf8Error);
    Io(io::Error);
    Serde(serde_json::Error);
  }

  errors {
    InvalidArg(expect: String) {
      description("invalid arguments"),
      display("invalid arguments: expected {}", expect),
    }

    ArgParse(msg: String) {
      description("argument parsing failed"),
      display("argument parsing failed: {}", msg),
    }
  }
}

error_chain! {
  types {
    ParseError, ParseErrorKind, ParseResultExt, ParseResult;
  }

  errors {
    NoMatch(s: String) {
      description("string matched nothing"),
      display("string '{}' matched nothing", s),
    }

    BadSyntax(s: String, expecting: String) {
      description("bad syntax"),
      display("bad syntax for '{}', expecting {}", s, expecting),
    }
  }
}

fn run() -> Result<()> {
  let mut args: VecDeque<_> = env::args().collect();
  args.pop_front(); // drop argv[0]

  fn parse_arg<T>(args: &mut VecDeque<String>, expect: &str) -> Result<T>
  where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::string::ToString,
  {
    match args.pop_front() {
      Some(a) => a,
      None => return Err(ErrorKind::InvalidArg(expect.into()).into()),
    }.parse()
      .map_err(|e: <T as std::str::FromStr>::Err| {
        ErrorKind::ArgParse(e.to_string()).into()
      })
  }

  let source: scrape::Source = parse_arg(&mut args, "a source selector")?;

  let retrieve: Box<Future<Item = Vec<u8>, Error = Error> + Send> = {
    use scrape::Source::*;

    match source {
      Reddit => {
        let subreddit: String = parse_arg(&mut args, "a subreddit name")?;
        let sort: read::SortType = parse_arg(&mut args, "a sort type")?;
        let limit: u32 = parse_arg(&mut args, "a post count")?;
        let pretty_outf: String = parse_arg(&mut args, "a filename")?;

        Box::new(scrape::reddit::scrape_subreddit(
          |id| {
            AppInfo::new_rc(
              id,
              "http://rk1024.net/".parse().unwrap(),
              AppDuration::Permanent,
              "read".split(" "),
              "linux",
              "rk1024/scrape-words",
              "v0.3.1",
              "rookie1024",
            )
          },
          subreddit,
          limit,
          sort,
          File::create(pretty_outf)?,
        )?)
      }
      Local => {
        let inf: String = parse_arg(&mut args, "a filename")?;

        Box::new(scrape::local::read(File::open(inf)?)?)
      }
    }
  };

  let proc: postprocess::Proc = parse_arg(&mut args, "a processing strategy")?;

  // TODO: remove this in favor of FnBox/Box<FnOnce>
  enum ProcessData {
    Analyze(File),
    Dump(File),
  }

  let process_data: ProcessData;

  let process: Box<
    Fn(Vec<u8>, ProcessData) -> Box<Future<Item = (), Error = Error> + Send>
      + Send
      + Sync,
  > = {
    use postprocess::Proc::*;

    match proc {
      Analyze => {
        let outf: String = parse_arg(&mut args, "a filename")?;

        process_data = ProcessData::Analyze(File::create(outf)?);

        Box::new(|vec, data| match data {
          ProcessData::Analyze(outf) => {
            Box::new(postprocess::analyze(vec, outf).into_future())
          }
          _ => unreachable!(),
        })
      }
      Dump => {
        let outf: String = parse_arg(&mut args, "a filename")?;

        process_data = ProcessData::Dump(File::create(outf)?);

        Box::new(|vec, data| match data {
          ProcessData::Dump(outf) => {
            Box::new(postprocess::dump(vec, outf).into_future())
          }
          _ => unreachable!(),
        })
      }
    }
  };

  tokio::run(
    retrieve
      .and_then(move |ins| process(ins, process_data))
      .map_err(|e: Error| {
        writeln!(io::stderr(), "an error occurred: {}", e).unwrap();
      }),
  );

  Ok(())
}

fn main() {
  match run() {
    Ok(_) => return,
    Err(e) => writeln!(io::stderr(), "an error occurred: {}", e).unwrap(),
  }
}
