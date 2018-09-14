use serde::{de, Deserialize, Deserializer};
use serde_json;
use std::{collections::HashMap, fmt, io::prelude::*};

pub struct WordList {
  forms: HashMap<String, WordlistForm>,
  set_keys: HashMap<usize, Vec<String>>,
  sets: HashMap<String, Vec<String>>,
}

impl WordList {
  pub fn new<R>(reader: R) -> Self
  where
    R: Read,
  {
    #[derive(Deserialize)]
    struct SerializedForm {
      forms: Option<HashMap<String, WordlistForm>>,
      sets: Option<HashMap<usize, HashMap<String, Vec<String>>>>,
    }

    let mut value: SerializedForm = serde_json::from_reader(reader).unwrap();

    let mut set_keys: HashMap<usize, Vec<String>> = HashMap::new();
    let mut sets: HashMap<String, Vec<String>> = HashMap::new();

    for (len, map) in value.sets.take().unwrap() {
      let mut keys = Vec::new();

      for (key, set) in map {
        keys.push(key.clone());
        sets.insert(key, set);
      }

      set_keys.insert(len, keys);
    }

    Self {
      forms: value.forms.take().unwrap(),
      set_keys,
      sets,
    }
  }

  pub fn get_form(&self, word: &str) -> Option<&WordlistForm> {
    self.forms.get(word)
  }

  pub fn get_set_keys(&self, len: &usize) -> Option<&Vec<String>> {
    self.set_keys.get(len)
  }

  pub fn get_set(&self, key: &str) -> Option<&Vec<String>> {
    self.sets.get(key)
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
