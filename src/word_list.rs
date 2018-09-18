use serde::{de, Deserialize, Deserializer};
use serde_json;
use std::{collections::HashMap, fmt, io::prelude::*};

#[derive(Deserialize)]
pub struct WordList {
  forms: Vec<(String, Vec<WordlistForm>)>, // (norm, [forms])
  sets: Vec<(String, Vec<usize>)>,         // [(depermuted, [form_idx])]
  set_keys: HashMap<usize, Vec<usize>>,    // len -> [set_idx]
}

impl WordList {
  pub fn new<R>(reader: R) -> Self
  where
    R: Read,
  {
    serde_json::from_reader(reader).unwrap()
  }

  pub fn get_form(&self, id: usize) -> Option<&(String, Vec<WordlistForm>)> {
    self.forms.get(id)
  }

  pub fn get_set_keys(&self, len: &usize) -> Option<&Vec<usize>> {
    self.set_keys.get(len)
  }

  pub fn get_set(&self, id: usize) -> Option<&(String, Vec<usize>)> {
    self.sets.get(id)
  }
}

pub struct WordlistForm {
  pub full: String,
  pub blanked: String,
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
