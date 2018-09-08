use regex::Regex;
use std::{
  collections::{BTreeSet, HashMap},
  fs::File,
  io::{self, prelude::*},
};
use unicode_normalization::UnicodeNormalization;
use Result;

lazy_static! {
  static ref WHITESPACE_RE: Regex = Regex::new(r"\s+").unwrap();
  static ref DROP_RE: Regex =
    Regex::new(r"^[\p{N}\p{M}\p{P}\p{Z}\p{C}]*$").unwrap();
  static ref NONWORD_TRIM_RE: Regex =
    Regex::new(r"(^[\W--\s]+|[\W--\s]+$)").unwrap();
  static ref NONWORD_STRIP_RE: Regex =
    Regex::new(r"[\p{M}\p{Ps}\p{Pe}\p{Pi}\p{Pf}\p{Po}\p{C}--/]").unwrap();
  static ref DEFER_SPLIT_RE: Regex =
    Regex::new(r"[/\p{Pc}\p{Pd}\p{Z}]").unwrap();
}

pub fn analyze(ins: Vec<u8>, mut outf: File) -> Result<()> {
  writeln!(io::stderr(), "post-processing text...")?;

  writeln!(io::stderr(), "  decoding UTF-8...")?;
  let mut string = String::from_utf8(ins)?;

  writeln!(io::stderr(), "  applying NFKD decomp...")?;
  string = string.nfkd().collect();

  let mut forms: HashMap<String, BTreeSet<String>> = HashMap::new();
  let mut counts: HashMap<String, usize> = HashMap::new();
  let mut defer: HashMap<String, usize> = HashMap::new();

  writeln!(io::stderr(), "  performing frequency analysis...")?;

  for word in WHITESPACE_RE.split(&string) {
    use std::collections::hash_map::Entry::*;

    let mut norm = NONWORD_TRIM_RE.replace_all(word, "").into_owned();
    norm = NONWORD_STRIP_RE.replace_all(&norm, "").into_owned();

    norm = norm.to_lowercase();

    match forms.entry(norm.clone()) {
      Vacant(v) => {
        v.insert(BTreeSet::new()).insert(word.into());
      }
      Occupied(mut o) => {
        o.get_mut().insert(word.into());
      }
    }

    if norm.len() == 0 {
      continue;
    }

    if DROP_RE.is_match(&norm) {
      writeln!(io::stdout(), "  DROP {:?}", norm)?;

      continue;
    }

    if DEFER_SPLIT_RE.is_match(&norm) {
      match defer.entry(norm.into()) {
        Vacant(v) => {
          writeln!(io::stdout(), "    deferring {:?}", v.key())?;
          v.insert(1);
        }
        Occupied(mut o) => {
          let v = o.get_mut();
          *v = *v + 1;
        }
      }
    } else {
      match counts.entry(norm.into()) {
        Vacant(v) => {
          v.insert(1);
        }
        Occupied(mut o) => {
          let v = o.get_mut();
          *v = *v + 1;
        }
      }
    }
  }

  writeln!(io::stderr(), "  processing deferred words...")?;

  let mut defer_counts: HashMap<String, usize> = HashMap::new();

  {
    #[derive(Debug)]
    enum Action {
      Split,
      Concat,
      Trim,
    }

    let mut actions: HashMap<String, Action> = HashMap::new();

    for (word, count) in defer.iter() {
      use std::collections::hash_map::Entry::*;

      let count = *count;

      let split: Vec<_> = DEFER_SPLIT_RE.split(&word).collect();

      let trimmed;

      // TODO: remove duplicate code

      if split.iter().all(|e| {
        let e = e.to_string(); // TODO: don't clone this
        counts.get(&e).unwrap_or(&0) + defer.get(&e).unwrap_or(&0) >= count
      }) {
        actions.insert(word.clone(), Action::Split);

        for word in split {
          match defer_counts.entry(word.into()) {
            Vacant(v) => {
              v.insert(count);
            }
            Occupied(mut o) => {
              let v = o.get_mut();
              *v = *v + count;
            }
          }
        }
      } else if {
        trimmed = split.concat();
        counts.get(&trimmed).unwrap_or(&0) > &count
      } {
        actions.insert(word.clone(), Action::Trim);

        match defer_counts.entry(trimmed) {
          Vacant(v) => {
            v.insert(count);
          }
          Occupied(mut o) => {
            let v = o.get_mut();
            *v = *v + count;
          }
        }
      } else {
        actions.insert(word.clone(), Action::Concat);

        match defer_counts.entry(word.clone()) {
          Vacant(v) => {
            v.insert(count);
          }
          Occupied(mut o) => {
            let v = o.get_mut();
            *v = *v + count;
          }
        }
      }
    }

    let mut sorted: Vec<_> = actions.iter().collect();

    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (word, action) in sorted {
      writeln!(io::stdout(), "{:16} {}", format!("{:?}", action), word)?;
    }
  }

  for (word, count) in defer_counts {
    use std::collections::hash_map::Entry::*;

    match counts.entry(word.into()) {
      Vacant(v) => {
        v.insert(count);
      }
      Occupied(mut o) => {
        let v = o.get_mut();
        *v = *v + count;
      }
    }
  }

  writeln!(io::stderr(), "  finalizing...")?;

  let mut sorted: Vec<_> = counts.iter().collect();

  sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

  writeln!(io::stderr(), "printing data...")?;

  for (i, (word, count)) in sorted.iter().enumerate() {
    let mut word = *word;
    let mut word_forms: Vec<_>;

    if let Some(f) = forms.get(word) {
      word_forms = f.iter().collect();
    } else {
      word_forms = Vec::new();
    }

    if word_forms.len() == 1 {
      word = word_forms.pop().unwrap();
    }

    if word_forms.is_empty() {
      writeln!(outf, "#{:6} ({:6}) : {}", i + 1, count, word)?;
    } else {
      writeln!(
        outf,
        "#{:6} ({:6}) : {:32} {}",
        i + 1,
        count,
        word,
        word_forms
          .iter()
          .filter(|f| f != &&word)
          .map(|w| String::clone(w))
          .collect::<Vec<_>>()
          .join("\t\t")
      )?;
    }
  }

  Ok(())
}

pub fn dump(ins: Vec<u8>, mut outf: File) -> Result<()> {
  io::copy(&mut ins.as_slice(), &mut outf)?;

  Ok(())
}
