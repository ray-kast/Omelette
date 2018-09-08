use futures::{future, prelude::*, stream};
use process_html;
use reddit::{self, auth, client, read, types};
use regex::Regex;
use serde_json;
use std::{
  boxed::Box,
  cmp,
  fs::File,
  io::{self, prelude::*},
  sync::{Arc, Mutex},
};
use {Error, Result};

struct ClientHooks;

impl client::ClientHooks for ClientHooks {
  fn auth_code_state(&self) -> String {
    "uwu".into()
  }

  fn save_token(
    &self,
    tok: auth::RcAuthToken,
  ) -> Box<dyn Future<Item = (), Error = ()> + Send> {
    Box::new(
      File::create("etc/apitok.json")
        .into_future()
        .from_err()
        .and_then(move |file| {
          writeln!(io::stderr(), "saving etc/apitok.json...").unwrap();

          serde_json::to_writer(file, tok.as_ref())
            .into_future()
            .from_err()
        })
        .map_err(|e: Error| {
          writeln!(io::stderr(), "failed to save etc/apitok.json: {}", e)
            .unwrap()
        }),
    )
  }
}

pub fn scrape_subreddit<F>(
  get_app: F,
  subreddit: String,
  limit: u32,
  sort: read::SortType,
  mut pretty_outf: File,
) -> Result<impl Future<Item = Vec<u8>, Error = Error>>
where
  F: FnOnce(reddit::AppId) -> reddit::RcAppInfo,
{
  let id = {
    let file = File::open("etc/apikey.json")?;

    serde_json::from_reader(file)?
  };

  let app = get_app(id);

  fn retrieve_token() -> Result<auth::AuthToken> {
    let file = File::open("etc/apitok.json")?;

    Ok(serde_json::from_reader(file)?)
  }

  let tok = match retrieve_token() {
    Ok(tok) => Some(tok),
    Err(e) => {
      writeln!(io::stderr(), "failed to get saved token: {}", e)?;
      None
    }
  };

  writeln!(io::stderr(), "initializing...")?;

  Ok(
    client::Client::new(Arc::new(ClientHooks), app, tok, 10)
      .from_err()
      .and_then(move |client| {
        writeln!(io::stderr(), "listing r/{}...", subreddit).unwrap();

        let listings = Arc::new(Mutex::new(Vec::<types::Listing>::new()));

        let client_1 = client.clone();
        let listings_1 = listings.clone();

        future::loop_fn((limit, None), move |(remain, after)| {
          let limit = cmp::min(remain, 100);
          let client = client_1.clone();
          let listings = listings_1.clone();

          writeln!(io::stderr(), "taking {} of {}", limit, remain).unwrap();

          read::list_subreddit(
            client.clone(),
            subreddit.clone(),
            sort,
            Some(limit),
            after,
          ).map(move |listing| {
            let mut listings = listings.lock().unwrap();

            let len = listing.children.len();

            if len > 0 {
              let name = listing.children[len - 1].as_link().name.clone();

              listings.push(listing);

              let remain = remain - limit;

              if remain > 0 {
                future::Loop::Continue((remain, Some(name)))
              } else {
                future::Loop::Break(())
              }
            } else {
              writeln!(io::stderr(), "stopping on empty listing").unwrap();
              future::Loop::Break(())
            }
          })
        }).from_err()
          .and_then(move |_| {
            let links: Vec<_>;

            {
              let mut listings = listings.lock().unwrap();
              links = listings.drain(0..).flat_map(|l| l.children).collect();
            }

            writeln!(io::stderr(), "retrieving comments...").unwrap();

            let total = links.len();

            // TODO: this could be atomic
            let n = Arc::new(Mutex::new(0));

            let tasks = links.iter().map(|child| {
              let n = n.clone();

              read::get_comments(client.clone(), child.as_link()).map(
                move |(links, comments)| {
                  {
                    let mut n = n.lock().unwrap();
                    *n = *n + 1;

                    for link in &links.as_listing().children {
                      write!(
                        io::stderr(),
                        "\r\x1b[2K  ({:4}/{:4}) {}",
                        n,
                        total,
                        link.as_link().url
                      ).unwrap();
                      io::stderr().flush().unwrap();
                    }
                  }

                  (links, comments)
                },
              )
            });

            stream::futures_ordered(tasks).collect().from_err()
          })
      })
      .map(move |vec| {
        let mut outs: Vec<u8> = Vec::new();

        writeln!(io::stderr(), "\nprocessing data...").unwrap();

        let total = vec.len();

        for (i, (links, comments)) in vec.iter().enumerate() {
          for link in &links.as_listing().children {
            write!(
              io::stderr(),
              "\r\x1b[2K  ({:4}/{:4}) {}",
              i + 1,
              total,
              link.as_link().url
            ).unwrap();
            io::stderr().flush().unwrap();

            match link {
              types::Thing::Link(l) => {
                print_link(&l, &mut pretty_outf).unwrap();
                dump_link(&l, &mut outs).unwrap();
              }
              _ => writeln!(pretty_outf, "## <unexpected Thing>").unwrap(),
            }
          }

          for comment in &comments.as_listing().children {
            match comment {
              types::Thing::Comment(c) => {
                print_comment(&c, "  ", &mut pretty_outf).unwrap();
                dump_comment(&c, &mut outs).unwrap();
              }
              types::Thing::More(_) => writeln!(pretty_outf, "  ...").unwrap(),
              _ => writeln!(pretty_outf, "  <unexpected Thing>").unwrap(),
            }
          }

          write!(pretty_outf, "\n\n\n").unwrap();
        }

        outs
      }),
  )
}

lazy_static! {
  static ref NEWLINE_RE: Regex = Regex::new(r"(\n+)").unwrap();
}

pub fn get_body_string(html: &str) -> Result<String> {
  let body = process_html::unwrap(html)?;

  Ok(process_html::pretty_unwrap(&body)?)
}

pub fn print_link<W>(link: &types::Link, outs: &mut W) -> Result<()>
where
  W: Write,
{
  writeln!(outs, "## ({}) {} ({})", link.score, link.title, link.url)?;

  writeln!(outs, "  ## u/{}", link.author)?;

  let body = link.selftext_html.clone().unwrap_or("".into());
  let body = get_body_string(&body)?;

  if body.len() != 0 {
    let body = NEWLINE_RE.replace_all(&body, "$1  # ");

    writeln!(outs, "  # {}", body)?;
  }

  Ok(())
}

pub fn print_comment<W>(
  comment: &types::Comment,
  indent: &str,
  outs: &mut W,
) -> Result<()>
where
  W: Write,
{
  let body = &comment.body_html;
  let body = get_body_string(&body)?;

  if NEWLINE_RE.is_match(&body) {
    let body = format!("\n{}", body);
    let body =
      NEWLINE_RE.replace_all(&body, format!("$1{}  : ", indent).as_str());

    writeln!(
      outs,
      "{}({:4}) u/{} ->{}",
      indent, comment.score, comment.author, body,
    )?;
  } else {
    writeln!(
      outs,
      "{}({:4}) u/{:32}: {}",
      indent, comment.score, comment.author, body,
    )?;
  }

  match &comment.replies {
    types::CommentReplies::Some(l) => {
      let l = l.as_listing();
      for reply in &l.children {
        match reply {
          types::Thing::Comment(c) => {
            print_comment(&c, &format!("{}  ", indent), outs)?
          }
          types::Thing::More(_) => writeln!(outs, "{}  ...", indent)?,
          _ => writeln!(outs, "{}<unexpected Thing>", indent)?,
        }
      }
    }
    types::CommentReplies::None => {}
  }

  Ok(())
}

// TODO: handle flair?

pub fn dump_link<W>(link: &types::Link, outs: &mut W) -> Result<()>
where
  W: Write,
{
  let body = link.selftext_html.clone().unwrap_or("".into());
  let body = get_body_string(&body)?;

  writeln!(outs, "{}", body)?;

  Ok(())
}

pub fn dump_comment<W>(comment: &types::Comment, outs: &mut W) -> Result<()>
where
  W: Write,
{
  let body = &comment.body_html;
  let body = get_body_string(&body)?;

  writeln!(outs, "{}", body)?;

  match &comment.replies {
    types::CommentReplies::Some(l) => {
      let l = l.as_listing();
      for reply in &l.children {
        match reply {
          types::Thing::Comment(c) => dump_comment(&c, outs)?,
          types::Thing::More(_) => (),
          _ => writeln!(outs, "<unexpected Thing>")?,
        }
      }
    }
    types::CommentReplies::None => (),
  }

  Ok(())
}
