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

  let mut dropped: BTreeSet<String> = BTreeSet::new();

  writeln!(io::stderr(), "  performing frequency analysis...")?;

  fn tick(map: &mut HashMap<String, usize>, word: String, count: usize) {
    use std::collections::hash_map::Entry::*;

    match map.entry(word) {
      Vacant(v) => {
        v.insert(count);
      }
      Occupied(mut o) => {
        let v = o.get_mut();
        *v = *v + count;
      }
    }
  }

  fn tick_set(
    map: &mut HashMap<String, BTreeSet<String>>,
    word: String,
    entry: String,
  ) {
    use std::collections::hash_map::Entry::*;

    match map.entry(word) {
      Vacant(v) => {
        v.insert(BTreeSet::new()).insert(entry);
      }
      Occupied(mut o) => {
        o.get_mut().insert(entry);
      }
    }
  }

  fn normalize_word(word: &str) -> String {
    let mut norm = NONWORD_TRIM_RE.replace_all(word, "").into_owned();
    norm = NONWORD_STRIP_RE.replace_all(&norm, "").into_owned();

    norm.to_lowercase()
  }

  fn should_drop(word: &str) -> bool {
    word.len() == 0 || DROP_RE.is_match(word)
  }

  for word in WHITESPACE_RE.split(&string) {
    if DEFER_SPLIT_RE.is_match(&word) {
      tick(&mut defer, word.into(), 1);

      continue;
    }

    let norm = normalize_word(word);

    if !should_drop(&norm) {
      tick_set(&mut forms, norm.clone(), word.into());
      tick(&mut counts, norm.into(), 1);
    } else {
      dropped.insert(word.into());
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
      let count = *count;

      let split: Vec<_> = DEFER_SPLIT_RE.split(&word).collect();

      let trimmed;

      // TODO: pick the most favorable action, not the first that matches
      if split.iter().all(|e| {
        let norm = normalize_word(e);
        should_drop(&norm)
          || (norm.len() >= 2 && counts.get(&norm).unwrap_or(&0) >= &count)
      }) {
        actions.insert(word.clone(), Action::Split);

        for word in split {
          let norm = normalize_word(word);

          if !should_drop(&norm) {
            tick_set(&mut forms, norm.clone(), word.into());

            tick(&mut defer_counts, norm, count);
          } else {
            dropped.insert(word.into());
          }
        }
      } else if {
        trimmed = normalize_word(&split.concat());
        counts.get(&trimmed).unwrap_or(&0) > &count
      } {
        actions.insert(word.clone(), Action::Trim);

        if !should_drop(&trimmed) {
          tick_set(&mut forms, trimmed.clone(), word.clone());

          tick(&mut defer_counts, trimmed, count);
        } else {
          dropped.insert(word.clone());
        }
      } else {
        actions.insert(word.clone(), Action::Concat);

        let norm = normalize_word(word);

        if !should_drop(&norm) {
          tick_set(&mut forms, norm.clone(), word.clone());

          tick(&mut defer_counts, norm, count);
        } else {
          dropped.insert(word.clone());
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
    tick(&mut counts, word, count);
  }

  writeln!(io::stderr(), "  finalizing...")?;

  let mut sorted: Vec<_> = counts.iter().collect();

  sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

  writeln!(io::stderr(), "printing data...")?;

  for word in dropped {
    writeln!(io::stdout(), "DROP {:?}", word)?;
  }

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
