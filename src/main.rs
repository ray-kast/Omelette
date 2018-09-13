extern crate ncurses as nc;
extern crate regex;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

mod tui;
mod word_list;

use regex::Regex;
use std::{collections::HashMap, fs::File, panic, rc::Rc};
use tui::{
  controls::*,
  element::{self as el, Element},
};

fn dump_line(win: nc::WINDOW, y: i32, line: &str) {
  nc::wmove(win, y, 0);
  nc::wclrtoeol(win);
  nc::mvwaddstr(win, y, 0, line);
  nc::wrefresh(win);
}

fn main() {
  panic::catch_unwind(|| {
    nc::endwin();
  }).unwrap();

  let _words = word_list::read_file(
    &mut File::open("words.json").expect("wordlist not found"),
  );

  let win = nc::initscr();
  nc::cbreak();
  nc::noecho();
  nc::keypad(win, true);

  let word_box = el::wrap(WordBox::new(8));

  let mut forms = Vec::new();
  let mut match_boxes: HashMap<String, _> = HashMap::new();

  for word in vec![""].into_iter() {
    lazy_static! {
      static ref BLANK_RE: Regex = Regex::new(r"\w").unwrap();
    }

    forms.push(word_list::WordlistForm {
      full: word.to_string(),
      blanked: BLANK_RE.replace_all(word, "_").into_owned(),
    });
  }

  for form in forms.iter() {
    match_boxes.insert(form.full.clone(), el::wrap(MatchBox::new(form)));
  }

  let match_box_panel = el::wrap(WrapBox::new(
    match_boxes.iter().map(|(_, b)| el::add_ref(b)),
    WrapMode::Cols,
    WrapAlign::Begin,
    1,
  ));

  let center_test = el::wrap(TestView::new(
    el::add_ref(&word_box),
    el::add_ref(&match_box_panel),
  ));

  let ui_root = UiRoot::new(win, el::add_ref(&center_test));

  ui_root.resize();

  loop {
    match nc::wgetch(win) {
      0x04 => break,                         // EOT
      0x17 => word_box.borrow_mut().clear(), // ETB
      0x0A => {
        // EOL
        let mut word_box = word_box.borrow_mut();

        match match_boxes.get(&word_box.buf) {
          Some(b) => {
            let mut b = b.borrow_mut();

            b.set_revealed(true);
          },
          None => (),
        }

        word_box.clear();
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
            dump_line(win, 3, &ch.escape_unicode().to_string());
            word_box.render_cur();
          }
        } else {
          dump_line(win, 4, &ch.to_string());
          word_box.render_cur();
        }
      }
    }
  }

  nc::endwin();
}
