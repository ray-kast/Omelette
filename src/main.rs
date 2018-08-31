extern crate ncurses as nc;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

mod tui;
mod word_list;

use std::fs::File;
use tui::{controls::*, element as el};

fn dump_line(win: nc::WINDOW, y: i32, line: &str) {
  nc::wmove(win, y, 0);
  nc::wclrtoeol(win);
  nc::mvwaddstr(win, y, 0, line);
  nc::wrefresh(win);
}

fn main() {
  let _words = word_list::read_file(
    &mut File::open("words.json").expect("wordlist not found"),
  );

  let win = nc::initscr();
  nc::cbreak();
  nc::noecho();
  nc::keypad(win, true);

  let word_box = el::wrap(WordBox::new(8));

  let test_form = word_list::WordlistForm{
    full: "abc".to_string(),
    blanked: "___".to_string(),
  };

  let match_box = el::wrap(MatchBox::new(&test_form));

  let center_test = el::wrap(TestView::new(
    el::add_ref(&word_box),
    el::add_ref(&match_box),
  ));

  let ui_root = UiRoot::new(win, el::add_ref(&center_test));

  ui_root.resize();

  loop {
    match nc::wgetch(win) {
      0x04 => break,                         // EOT
      0x17 => word_box.borrow_mut().clear(), // ETB
      0x0A => {
        // EOL
        let mut match_box = match_box.borrow_mut();
        let mut word_box = word_box.borrow_mut();

        if word_box.buf == "abc" {
          match_box.set_revealed(true);
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

  nc::endwin(); // TODO: should I worry about panicking?
}
