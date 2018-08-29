extern crate ncurses as nc;

mod tui;

use tui::{core::*, element::*, word_box::*};

fn dump_line(win: nc::WINDOW, y: i32, line: &str) {
  nc::wmove(win, y, 0);
  nc::wclrtoeol(win);
  nc::mvwaddstr(win, y, 0, line);
  nc::wrefresh(win);
}

fn main() {
  let win = nc::initscr();
  nc::cbreak();
  nc::noecho();
  nc::keypad(win, true);

  let word_box_win = nc::newwin(1, 15, 1, 1);

  let mut word_box = WordBox::new(word_box_win, 7);

  nc::wrefresh(win);

  let mut termsize = Size { w: 0, h: 0 };
  nc::getmaxyx(win, &mut termsize.h, &mut termsize.w);

  word_box.measure(termsize);

  {
    let space = Rect {
      pos: Point { x: 1, y: 1 },
      size: word_box.desired_size(),
    };

    word_box.arrange(space);
  }

  word_box.render();

  loop {
    let ch = nc::wgetch(win);

    match ch {
      0x04 => break,            // EOT
      0x17 => word_box.clear(), // ETB
      0x0A => {
        // EOL
        dump_line(win, 5, &word_box.buf);
        word_box.clear();
      }
      0x7F => word_box.del_left(), // DEL
      nc::KEY_LEFT => word_box.left(),
      nc::KEY_RIGHT => word_box.right(),
      nc::KEY_HOME => word_box.home(),
      nc::KEY_BACKSPACE => word_box.del_left(),
      nc::KEY_DC => word_box.del_right(),
      nc::KEY_END => word_box.end(),
      nc::KEY_RESIZE => word_box.render(),
      _ => {
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
