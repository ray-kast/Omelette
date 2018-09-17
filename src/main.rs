extern crate ncurses as nc;
extern crate rand;
extern crate regex;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

mod tui;
mod word_list;

use rand::prelude::*;
use regex::Regex;
use std::{
  collections::HashMap,
  fs::File,
  io::{self, prelude::*},
  panic,
};
use tui::{
  controls::*,
  element::{self as el, Element},
};
use word_list::WordList;

fn dump_line(win: nc::WINDOW, y: i32, line: &str) {
  nc::wmove(win, y, 0);
  nc::wclrtoeol(win);
  nc::mvwaddstr(win, y, 0, line);
  nc::wrefresh(win);
}

fn count_chars(s: &str) -> HashMap<char, usize> {
  let mut ret = HashMap::new();

  for c in s.chars() {
    use std::collections::hash_map::Entry::*;

    match ret.entry(c) {
      Vacant(v) => {
        v.insert(1);
      }
      Occupied(o) => {
        let mut val = o.into_mut();
        *val = *val + 1;
      }
    }
  }

  ret
}

fn main() {
  panic::catch_unwind(|| {
    nc::endwin();
  }).unwrap();

  let words = WordList::new(
    &mut File::open("etc/words.json").expect("wordlist not found"),
  );

  loop {
    let key;
    let set = {
      let mut len: usize;
      let keys = loop {
        let mut len_str = String::new();

        write!(io::stderr(), "word length: ").unwrap();
        io::stderr().flush().unwrap();

        if io::stdin().read_line(&mut len_str).unwrap() == 0 {
          writeln!(io::stderr(), "").unwrap();
          return;
        }

        len = match len_str.trim().parse() {
          Ok(l) => l,
          Err(e) => {
            writeln!(io::stderr(), "invalid number: {}", e).unwrap();
            continue;
          }
        };

        match words.get_set_keys(&len) {
          Some(k) => break k,
          None => {
            writeln!(io::stderr(), "no words found of length {}", len).unwrap();
            continue;
          }
        }
      };

      key = &keys[rand::thread_rng().gen_range(0, keys.len())];

      words.get_set(key).unwrap()
    };

    let win = nc::initscr();
    nc::start_color();
    nc::cbreak();
    nc::noecho();
    nc::keypad(win, true);

    let ghost_pair: i32 = 1;
    nc::init_pair(ghost_pair as i16, 2, 0);
    // nc::init_extended_pair(ghost_pair, 2, 0);

    let hl_pair: i32 = 2;
    nc::init_pair(hl_pair as i16, 2, 0);

    let word_box = el::wrap(WordBox::new(key.clone(), ghost_pair));

    let mut match_boxes: HashMap<String, _> = HashMap::new();

    for word in set {
      let form = words.get_form(word).unwrap();
      match_boxes
        .insert(word.to_string(), el::wrap(MatchBox::new(form, hl_pair)));
    }

    let match_box_panel = el::wrap(WrapBox::new(
      set.iter().map(|f| el::add_ref(&match_boxes[f])),
      WrapMode::Cols,
      WrapAlign::Begin,
      3,
    ));

    let mut hl_match_box: Option<el::ElemWrapper<MatchBox>> = None;

    let center_test = el::wrap(TestView::new(
      el::add_ref(&word_box),
      el::add_ref(&match_box_panel),
    ));

    let ui_root = UiRoot::new(win, el::add_ref(&center_test));

    ui_root.resize();

    loop {
      match nc::wgetch(win) {
        0x04 => {
          nc::endwin(); // TODO: break out of the outer loop instead
          return;
        } // EOT
        0x09 => word_box.borrow_mut().shuffle(), // HT
        0x17 => word_box.borrow_mut().clear(),   // ETB
        // TODO: capture escape sequences
        0x1B => break, // ESC
        0x0A => {
          // EOL
          match hl_match_box {
            Some(ref b) => {
              let mut b = b.borrow_mut();

              b.set_highlighted(false);
            }
            None => {}
          }

          {
            let mut word_box = word_box.borrow_mut();

            match match_boxes.get(word_box.buf()) {
              Some(b) => {
                let mut b_ref = b.borrow_mut();

                if b_ref.revealed() {
                  b_ref.set_highlighted(true);

                  hl_match_box = Some(b.clone());
                } else {
                  b_ref.set_revealed(true);
                }
              }
              None => (),
            }

            word_box.clear();
          }
        }
        0x7F => word_box.borrow_mut().del_left(), // DEL
        nc::KEY_LEFT => word_box.borrow_mut().left(),
        nc::KEY_RIGHT => word_box.borrow_mut().right(),
        nc::KEY_HOME => word_box.borrow_mut().home(),
        nc::KEY_BACKSPACE => word_box.borrow_mut().del_left(),
        nc::KEY_DC => word_box.borrow_mut().del_right(),
        nc::KEY_END => word_box.borrow_mut().end(),
        nc::KEY_RESIZE => ui_root.resize(),
        ch => {
          let mut word_box = word_box.borrow_mut();

          if ch < nc::KEY_MIN {
            let ch = ch as u8 as char;

            if !ch.is_control() {
              let s = ch.to_lowercase().to_string();
              word_box.put(&s);
            } else {
              // dump_line(win, 3, &ch.escape_unicode().to_string());
              // word_box.render_cur();
            }
          } else {
            // dump_line(win, 4, &ch.to_string());
            // word_box.render_cur();
          }
        }
      }
    }

    nc::endwin();
  }
}
