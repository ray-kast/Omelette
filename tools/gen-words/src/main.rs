extern crate regex;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

mod thread_pool;

use regex::Regex;
use std::{
  collections::{HashMap, HashSet},
  fs::File,
  io::{self, prelude::*, BufReader, BufWriter},
  str,
  sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{channel, Sender},
    Arc,
  },
  time::Instant
};
use thread_pool::ThreadPool;

#[derive(Clone, Serialize)]
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

static MIN_VALID_LEN: usize = 3;
static MIN_LEN: usize = 4;
static MAX_LEN: usize = 8;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Normalized(String); // Used as a string with nonword character stripped
#[derive(Clone, PartialEq, Eq, Hash)]
struct Depermuted(String); // Used as a Normalized with its characters sorted

struct Stage1 {
  permutations: HashMap<Depermuted, HashSet<Normalized>>,
  counts: HashMap<Depermuted, CharCounts>,
  valid_subwords: HashSet<Depermuted>,
  len_groups: HashMap<usize, HashSet<Depermuted>>,
  forms: HashMap<Normalized, Vec<WordlistForm>>,
}

struct Stage2<'a> {
  sets: HashMap<Depermuted, Vec<Normalized>>, // TODO: can I go back to borrowing inside the vec?
  set_keys: HashMap<usize, Vec<&'a Depermuted>>,
  used_words: HashSet<Normalized>,
}

fn stage_1() -> Stage1 {
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

  let mut permutations: HashMap<Depermuted, HashSet<Normalized>> =
    HashMap::new();
  let mut counts: HashMap<Depermuted, CharCounts> = HashMap::new();
  let mut len_groups: HashMap<usize, HashSet<Depermuted>> = HashMap::new();
  let mut valid_subwords: HashSet<Depermuted> = HashSet::new();

  let mut forms: HashMap<Normalized, Vec<WordlistForm>> = HashMap::new();

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

        if depermuted.0.len() >= MIN_VALID_LEN {
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

  Stage1 {
    permutations,
    counts,
    valid_subwords,
    len_groups,
    forms,
  }
}

fn stage_2<'a>(s1: &'a Arc<Stage1>) -> Stage2<'a> {
  let mut sets: HashMap<Depermuted, Vec<Normalized>> = HashMap::new(); // TODO: can I go back to borrowing inside the vec?
  let mut set_keys: HashMap<usize, Vec<&Depermuted>> = HashMap::new();

  let mut used_words: HashSet<Normalized> = HashSet::new();

  let (set_tx, set_rx) = channel();

  let done = Arc::new(AtomicUsize::new(0));

  {
    let worker: ThreadPool<_> = ThreadPool::new(
      (0..10)
        .map(|_| (Arc::clone(s1), done.clone(), set_tx.clone()))
        .collect(),
      |_id,
       (s1, done, set_tx): &(Arc<Stage1>, Arc<AtomicUsize>, Sender<_>),
       (depermuted, count): (Depermuted, CharCounts)| {
        let i = done.fetch_add(1, Ordering::Relaxed);
        if i % 10 == 0 {
          print!("\r\x1b[2K({}) {}", i, &depermuted.0);
          io::stdout().flush().unwrap();
        }

        let mut list: Vec<_> = s1
          .valid_subwords
          .iter()
          .filter(|deperm2| {
            deperm2.0.len() <= depermuted.0.len()
              && is_subseq(&s1.counts[*deperm2], &count)
          })
          .flat_map(|d| s1.permutations[d].clone()) // TODO: can I go back to borrowing this?
          .collect();

        list.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then(a.0.cmp(&b.0)));

        set_tx
          .send((depermuted, list))
          .expect("failed to send result");
      },
    );

    for len in MIN_LEN..MAX_LEN + 1 {
      let mut keys: Vec<&Depermuted> = Vec::new();

      for (_, depermuted) in s1.len_groups[&len].iter().enumerate() {
        worker.queue((depermuted.clone(), s1.counts[depermuted].clone()));
        keys.push(depermuted);
      }

      set_keys.insert(len, keys);
    }

    worker.join();

    println!();
  }

  for (depermuted, list) in set_rx.try_iter() {
    for norm in &list {
      used_words.insert(Normalized::clone(norm));
    }

    sets.insert(depermuted, list);
  }

  Stage2 {
    sets,
    set_keys,
    used_words,
  }
}

fn main() {
  let s1 = Arc::new(stage_1());

  let s2 = stage_2(&s1);

  let mut forms = s1.forms.clone();

  forms.retain(|k, _| s2.used_words.contains(k));

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

      set_vec = s2
        .sets
        .into_iter()
        .map(|(d, n)| (d, n.into_iter().map(|n| forms[&n]).collect()))
        .collect();

      let sets: HashMap<_, _> = set_vec
        .iter()
        .enumerate()
        .map(|(i, (d, _))| (d, i))
        .collect();

      set_key_map = s2
        .set_keys
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
