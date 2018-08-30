extern crate ncurses as nc;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

mod tui;
mod word_list;

use std::{cell::RefCell, fs::File, rc::Rc};
use tui::{center_test::*, core::*, element::*, word_box::*};

fn dump_line(win: nc::WINDOW, y: i32, line: &str) {
  nc::wmove(win, y, 0);
  nc::wclrtoeol(win);
  nc::mvwaddstr(win, y, 0, line);
  nc::wrefresh(win);
}

fn rearrange_root(win: nc::WINDOW, el: &mut dyn Element) {
  let mut termsize = Size { w: 0, h: 0 };
  nc::getmaxyx(win, &mut termsize.h, &mut termsize.w);

  el.measure(termsize);

  {
    let space = Rect {
      pos: Point { x: 0, y: 0 },
      size: termsize,
    };

    el.arrange(space);
  }

  el.render();
}

fn main() {
  let _words = word_list::read_file(
    &mut File::open("words.json").expect("wordlist not found"),
  );

  let win = nc::initscr();
  nc::cbreak();
  nc::noecho();
  nc::keypad(win, true);

  let word_box = Rc::new(RefCell::new(WordBox::new(8)));

  let mut center_test = CenterTest::new(Rc::clone(&word_box) as ElemRef);

  nc::wrefresh(win);

  rearrange_root(win, &mut center_test);

  loop {
    let ch = nc::wgetch(win);

    match ch {
      0x04 => break,                         // EOT
      0x17 => word_box.borrow_mut().clear(), // ETB
      0x0A => {
        // EOL
        let mut word_box = word_box.borrow_mut();
        dump_line(win, 5, &word_box.buf);
        word_box.clear();
      }
      0x7F => word_box.borrow_mut().del_left(), // DEL
      nc::KEY_LEFT => word_box.borrow_mut().left(),
      nc::KEY_RIGHT => word_box.borrow_mut().right(),
      nc::KEY_HOME => word_box.borrow_mut().home(),
      nc::KEY_BACKSPACE => word_box.borrow_mut().del_left(),
      nc::KEY_DC => word_box.borrow_mut().del_right(),
      nc::KEY_END => word_box.borrow_mut().end(),
      nc::KEY_RESIZE => {
        nc::wclear(win);
        nc::wrefresh(win);
        rearrange_root(win, &mut center_test);
      }
      _ => {
        if ch < nc::KEY_MIN {
          let ch = ch as u8 as char;

          if !ch.is_control() {
            let s = ch.to_lowercase().to_string();
            word_box.borrow_mut().put(&s);
          } else {
            dump_line(win, 3, &ch.escape_unicode().to_string());
            word_box.borrow_mut().render_cur();
          }
        } else {
          dump_line(win, 4, &ch.to_string());
          word_box.borrow_mut().render_cur();
        }
      }
    }
  }

  nc::endwin(); // TODO: should I worry about panicking?
}
