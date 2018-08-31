extern crate serde;
extern crate serde_json;

use serde::{de, Deserialize, Deserializer};
use std::{collections::HashMap, fmt, fs::File, io::prelude::*};

#[derive(Deserialize)]
pub struct WordList {
  forms: HashMap<String, WordlistForm>,
  sets: HashMap<usize, HashMap<String, Vec<String>>>,
}

pub struct WordlistForm {
  full: String,
  blanked: String,
}

struct WordlistFormVisitor();

impl<'de> de::Visitor<'de> for WordlistFormVisitor {
  type Value = WordlistForm;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    write!(formatter, "a wordlist form")
  }

  fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
  where
    A: de::SeqAccess<'de>,
  {
    let mut seq = seq;

    let full = match seq.next_element().expect("seq.next_element failed") {
      Some(val) => val,
      None => return Err(de::Error::invalid_length(0, &"a two-item array")),
    };

    let blanked = match seq.next_element().expect("seq.next_element failed") {
      Some(val) => val,
      None => return Err(de::Error::invalid_length(1, &"a two-item array")),
    };

    Ok(WordlistForm { full, blanked })
  }
}

impl<'de> Deserialize<'de> for WordlistForm {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_tuple(3, WordlistFormVisitor())
  }
}

pub fn read(data: &str) -> WordList {
  serde_json::from_str(data).expect("failed to parse wordlist")
}

pub fn read_file(file: &mut File) -> WordList {
  let mut data = String::new();
  file
    .read_to_string(&mut data)
    .expect("failed to read wordlist");

  read(&data)
}
