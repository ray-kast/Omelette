extern crate regex;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use regex::Regex;
use std::{
  collections::{HashMap, HashSet},
  fs::File,
  io,
  io::{prelude::*, BufReader, BufWriter},
  str,
  time::Instant,
};

#[derive(Serialize)]
struct WordlistForm(String, String); // (blanked, full)

#[derive(Serialize)]
struct Wordlist {
  forms: Vec<(String, Vec<WordlistForm>)>, // (norm, [forms])
  sets: Vec<(String, Vec<usize>)>,         // [(depermuted, [form_idx])]
  set_keys: HashMap<usize, Vec<usize>>,    // len -> [set_idx]
}

type CharCounts = HashMap<char, usize>;

fn count_chars(s: &str) -> CharCounts {
  let mut ret = CharCounts::new();

  for c in s.chars() {
    use std::collections::hash_map::Entry::*;

    match ret.entry(c) {
      Occupied(o) => {
        let mut val = o.into_mut();
        *val = *val + 1;
      }
      Vacant(v) => {
        v.insert(1);
      }
    }
  }

  ret
}

fn is_subseq(count: &CharCounts, of: &CharCounts) -> bool {
  count.iter().all(|(c, n)| n <= of.get(c).unwrap_or(&0))
}

fn main() {
  let min_valid_len = 3;
  let min_len = 4;
  let max_len = 8;

  let mut words = Vec::new();

  {
    let file = File::open("etc/wordlist.txt").expect("wordlist not found");
    let file = BufReader::new(&file);

    for line in file.lines() {
      let line = line.expect("failed to read line");

      words.push(line.trim().to_string());
    }
  }

  println!("read {} word(s)", words.len());

  #[derive(Clone, PartialEq, Eq, Hash)]
  struct Normalized(String); // Used as a string with nonword character stripped
  #[derive(Clone, PartialEq, Eq, Hash)]
  struct Depermuted(String); // Used as a Normalized with its characters sorted

  let mut permutations: HashMap<Depermuted, HashSet<Normalized>> =
    HashMap::new();
  let mut counts: HashMap<Depermuted, CharCounts> = HashMap::new();
  let mut len_groups: HashMap<usize, HashSet<Depermuted>> = HashMap::new();
  let mut valid_subwords: HashSet<Depermuted> = HashSet::new();
  let mut used_words: HashSet<Normalized> = HashSet::new();

  let mut forms: HashMap<Normalized, Vec<WordlistForm>> = HashMap::new();
  let mut sets: HashMap<Depermuted, Vec<&Normalized>> = HashMap::new();
  let mut set_keys: HashMap<usize, Vec<&Depermuted>> = HashMap::new();

  lazy_static! {
    static ref REJECT_RE: Regex = Regex::new(r"[\d\s]").unwrap();
    static ref NORMAL_RE: Regex = Regex::new(r"\W+").unwrap();
    static ref BLANK_RE: Regex = Regex::new(r"[\w--\p{Lu}\p{Lt}]").unwrap();
    static ref BLANK_CAPS_RE: Regex = Regex::new(r"[\p{Lu}\p{Lt}]").unwrap();
  }

  for word in words {
    use std::collections::hash_map::Entry::*;

    if REJECT_RE.is_match(&word) {
      continue;
    }

    let normalized = word.to_lowercase();
    let normalized =
      Normalized(NORMAL_RE.replace_all(&normalized, "").into_owned());

    let blank = BLANK_RE.replace_all(&word, "_");
    let blank = BLANK_CAPS_RE.replace_all(&blank, "_").into_owned(); // TODO: highlight this somehow?

    match forms.entry(normalized.clone()) {
      Vacant(v) => v.insert(Vec::new()),
      Occupied(o) => o.into_mut(),
    }.push(WordlistForm(word.clone(), blank));

    let mut depermuted: Vec<_> = normalized.0.chars().collect();
    depermuted.sort();
    let depermuted = Depermuted(depermuted.into_iter().collect());

    match permutations.entry(depermuted.clone()) {
      Vacant(v) => {
        v.insert(HashSet::new()).insert(normalized);
        counts.insert(depermuted.clone(), count_chars(&depermuted.0));

        match len_groups.entry(depermuted.0.len()) {
          Vacant(v) => {
            v.insert(HashSet::new()).insert(depermuted.clone());
          }
          Occupied(o) => {
            o.into_mut().insert(depermuted.clone());
          }
        }

        if depermuted.0.len() >= min_valid_len {
          valid_subwords.insert(depermuted);
        }
      }
      Occupied(o) => {
        o.into_mut().insert(normalized);
      }
    }
  }

  println!("{} normalized", forms.len());
  println!("{} depermuted", permutations.len());
  println!("{} valid subword(s)", valid_subwords.len());

  for len in min_len..max_len + 1 {
    println!("processing {}-letter words...", len);

    let start = Instant::now();

    let mut keys: Vec<&Depermuted> = Vec::new();

    let iter = counts.iter().filter(|(d, _)| d.0.len() == len);

    let total = iter.clone().count();

    for (i, (depermuted, count)) in iter.enumerate() {
      if i % 10 == 0 {
        print!("\r\x1b[2K({}/{}) {}", &i, &total, &depermuted.0);
        io::stdout().flush().unwrap();
      }

      let mut list: Vec<_> = valid_subwords
        .iter()
        .filter(|deperm2| {
          deperm2.0.len() <= depermuted.0.len()
            && is_subseq(&counts[*deperm2], &count)
        })
        .flat_map(|d| &permutations[d])
        .collect();

      list.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then(a.0.cmp(&b.0)));

      for norm in &list {
        used_words.insert(Normalized::clone(norm));
      }

      sets.insert(depermuted.clone(), list);
      keys.push(depermuted);
    }

    let end = Instant::now();
    let time = end - start;

    println!(
      "\r\x1b[2K{} processed in {}.{:02}s",
      total,
      time.as_secs(),
      time.subsec_millis() / 10
    );

    set_keys.insert(len, keys);
  }

  forms.retain(|k, _| used_words.contains(k));

  let list = {
    let form_vec: Vec<(Normalized, Vec<WordlistForm>)>;
    let set_vec: Vec<(Depermuted, Vec<usize>)>;
    let set_key_map: HashMap<usize, Vec<usize>>;

    {
      form_vec = forms.into_iter().collect();

      let forms: HashMap<_, _> = form_vec
        .iter()
        .enumerate()
        .map(|(i, (n, _))| (n, i))
        .collect();

      set_vec = sets
        .into_iter()
        .map(|(d, n)| (d, n.into_iter().map(|n| forms[n]).collect()))
        .collect();

      let sets: HashMap<_, _> = set_vec
        .iter()
        .enumerate()
        .map(|(i, (d, _))| (d, i))
        .collect();

      set_key_map = set_keys
        .into_iter()
        .map(|(l, d)| (l, d.into_iter().map(|d| sets[d]).collect()))
        .collect();
    }

    Wordlist {
      forms: form_vec.into_iter().map(|(n, f)| (n.0, f)).collect(),
      sets: set_vec.into_iter().map(|(d, n)| (d.0, n)).collect(),
      set_keys: set_key_map,
    }
  };

  {
    let file =
      File::create("etc/words.json").expect("couldn't create output file");
    let file = BufWriter::new(&file);

    serde_json::to_writer(file, &list).expect("failed to write JSON");
  }
}
