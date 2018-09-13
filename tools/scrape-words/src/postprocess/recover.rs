use regex::Regex;
use std::{
  collections::HashMap,
  io::{self, prelude::*},
};
use Result;

lazy_static! {
  static ref TRIM_RE: Regex = Regex::new(
    r"^[\p{M}\p{Pc}\p{Pd}\p{Ps}\p{Pi}\p{Po}\p{Sk}\p{Z}\p{C}]+|[\p{Pc}\p{Pd}\p{Pe}\p{Pf}\p{Po}\p{Sk}\p{Z}\p{C}]+$"
  ).unwrap();
}

// TODO: collect multiples rather than deduplicating
pub fn create_wordlist<'a, I, N, D>(
  list: I,
  normalize: N,
  dedup: D,
) -> Result<HashMap<String, String>>
where
  I: IntoIterator<Item = &'a str>,
  N: Fn(&str) -> String,
  D: for<'b> Fn(&'b str, &'b str) -> &'b str,
{
  let mut ret = HashMap::new();

  for word in list {
    let norm = normalize(word);

    use std::collections::hash_map::Entry::*;

    match ret.entry(norm) {
      Vacant(v) => {
        v.insert(word);
      }
      Occupied(mut o) => {
        let v = o.get_mut();

        *v = dedup(v, word);
      }
    }
  }

  let ret = ret.into_iter().map(|(k, v)| (k, v.to_string())).collect();

  Ok(ret)
}

pub fn create_map(
  forms: &HashMap<String, HashMap<String, usize>>,
  wordlist: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
  let mut ret: HashMap<String, String> = HashMap::new();

  for (word, word_forms) in forms {
    ret.insert(
      word.clone(),
      match wordlist.get(word) {
        Some(w) => w.clone(),
        None => {
          let val = word_forms
            .iter()
            .fold((None, &0), |(aw, ac), (xw, xc)| {
              // TODO: aggregate ties
              if xc > ac {
                (Some(xw), xc)
              } else {
                (aw, ac)
              }
            })
            .0
            .unwrap();

          let trimmed = TRIM_RE.replace_all(val, "").into_owned();

          if &trimmed != val {
            writeln!(
              io::stdout(),
              "trimming recovery form {:?} to {:?}",
              val,
              trimmed
            )?;
          }

          trimmed
        }
      },
    );
  }

  let ret = ret
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

  Ok(ret)
}
