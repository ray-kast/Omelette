use diesel::{prelude::*, sqlite::SqliteConnection};
use models::*;

pub struct WordList {
  conn: SqliteConnection,
}

impl WordList {
  pub fn new(url: &str) -> Self {
    Self {
      conn: SqliteConnection::establish(url).unwrap(),
    }
  }

  pub fn get_form(&self, key: &str) -> Vec<WordlistForm> {
    let id_results = {
      use schema::form_ids::dsl::*;

      form_ids
        .filter(norm.eq(key))
        .limit(1)
        .load::<FormIdQ>(&self.conn)
        .unwrap()
    };

    assert!(id_results.len() <= 1);

    let id_key = match id_results.first() {
      None => return Vec::new(),
      Some(i) => i,
    };

    let form_results = {
      use schema::forms::dsl::*;

      forms
        .filter(id.eq(id_key.id))
        .load::<FormQ>(&self.conn)
        .unwrap()
    };

    form_results
      .into_iter()
      .map(|r| WordlistForm {
        blanked: r.blank,
        full: r.full,
      })
      .collect()
  }

  pub fn get_set_keys(&self, len_key: &usize) -> Vec<String> {
    let results = {
      use schema::set_keys::dsl::*;

      set_keys
        .filter(len.eq(*len_key as i32))
        .load::<SetKeyQ>(&self.conn)
        .unwrap()
    };

    results.into_iter().map(|r| r.key).collect()
  }

  pub fn get_set(&self, key_str: &str) -> Vec<String> {
    let id_results = {
      use schema::set_ids::dsl::*;

      set_ids
        .filter(key.eq(key_str))
        .limit(1)
        .load::<SetIdQ>(&self.conn)
        .unwrap()
    };

    assert!(id_results.len() <= 1);

    let id_key = match id_results.first() {
      None => return Vec::new(),
      Some(i) => i,
    };

    let set_results = {
      use schema::sets::dsl::*;

      sets
        .filter(id.eq(id_key.id))
        .load::<SetQ>(&self.conn)
        .unwrap()
    };

    set_results.into_iter().map(|r| r.norm).collect()
  }
}

pub struct WordlistForm {
  pub full: String,
  pub blanked: String,
}
