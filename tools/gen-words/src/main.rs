extern crate regex;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use regex::Regex;
use std::{
  collections::{HashMap, HashSet},
  fs::File,
  io,
  io::{prelude::*, BufReader, BufWriter},
  str,
  time::{Duration, Instant},
};

#[derive(Serialize)]
struct WordlistForm(String, String);

#[derive(Serialize)]
struct Wordlist<'a> {
  forms: HashMap<String, WordlistForm>,
  sets: HashMap<usize, HashMap<String, Vec<&'a String>>>,
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

  // depermuted => [normalized]
  let mut permutations: HashMap<String, Vec<String>> = HashMap::new();
  // depermuted => count
  let mut counts: HashMap<String, CharCounts> = HashMap::new();
  // len => [depermuted]
  let mut len_groups: HashMap<usize, Vec<String>> = HashMap::new();
  // [depermuted]
  let mut valid_subwords: Vec<String> = Vec::new();
  // [normalized]
  let mut used_words: HashSet<String> = HashSet::new();

  let mut list = Wordlist {
    forms: HashMap::new(),
    sets: HashMap::new(),
  };

  let reject_re = Regex::new(r"[\d\s]").unwrap();
  let normal_re = Regex::new(r"\W+").unwrap();
  let blank_re = Regex::new(r"\w").unwrap();

  for word in words {
    use std::collections::hash_map::Entry::*;

    if reject_re.is_match(&word) {
      continue;
    }

    let normalized = word.to_lowercase();
    let normalized = normal_re.replace_all(&normalized, "").into_owned();

    match list.forms.entry(normalized.clone()) {
      Vacant(v) => {
        v.insert(WordlistForm(
          word.clone(),
          blank_re.replace_all(&word, "_").into_owned(),
        ));
      }
      Occupied(o) => {
        let val = o.get();
        println!(
          "WARNING: {} is a duplicate! ({} vs. {})",
          &normalized, &word, val.0
        );
      }
    }

    let mut depermuted: Vec<char> = normalized.chars().collect();
    depermuted.sort();
    let depermuted: String = depermuted.iter().collect();

    match permutations.entry(depermuted.clone()) {
      Vacant(v) => {
        v.insert([normalized].to_vec());
        counts.insert(depermuted.clone(), count_chars(&depermuted));

        match len_groups.entry(depermuted.len()) {
          Vacant(v) => {
            v.insert([depermuted.clone()].to_vec());
          }
          Occupied(o) => {
            o.into_mut().push(depermuted.clone());
          }
        }

        if depermuted.len() >= min_valid_len {
          valid_subwords.push(depermuted.clone());
        }
      }
      Occupied(o) => {
        o.into_mut().push(normalized.clone());
      }
    };
  }

  println!("{} normalized", list.forms.len());
  println!("{} depermuted", permutations.len());
  println!("{} valid subword(s)", valid_subwords.len());

  for len in min_len..max_len + 1 {
    println!("processing {}-letter words...", len);

    let start = Instant::now();

    let mut sets: HashMap<String, Vec<&String>> = HashMap::new();

    let iter = counts.iter().filter(|(d, _)| d.len() == len);

    let total = iter.clone().count();

    for (i, (depermuted, count)) in iter.enumerate() {
      if i % 10 == 0 {
        print!("\r\x1b[2K({}/{}) {}", &i, &total, &depermuted);
        io::stdout().flush().unwrap();
      }

      let mut list: Vec<&String> = valid_subwords
        .iter()
        .filter(|deperm2| {
          deperm2.len() <= depermuted.len()
            && is_subseq(&counts[*deperm2], &count)
        })
        .flat_map(|d| &permutations[d])
        .collect();

      list.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(&b)));

      for norm in list.iter() {
        used_words.insert(String::clone(norm));
      }

      sets.insert(depermuted.clone(), list);
    }

    let end = Instant::now();
    let time = end - start;

    println!(
      "\r\x1b[2K{} processed in {}.{:02}s",
      total,
      time.as_secs(),
      time.subsec_millis() / 10
    );

    list.sets.insert(len, sets);
  }

  list.forms.retain(|k, _| used_words.contains(k));

  {
    let file =
      File::create("etc/words.json").expect("couldn't create output file");
    let file = BufWriter::new(&file);

    serde_json::to_writer(file, &list).expect("failed to write JSON");
  }
}
