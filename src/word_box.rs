extern crate ncurses as nc;

use std::cmp;

pub struct WordBox {
  win: nc::WINDOW,
  cur: usize,
  max: usize,
  pub buf: String,
}

impl WordBox {
  pub fn new(win: nc::WINDOW, max: usize) -> WordBox {
    return WordBox {
      win: win,
      cur: 0,
      max: max,
      buf: String::new(),
    };
  }

  pub fn render_cur(&self) {
    nc::wmove(self.win, 0, (self.cur * 2) as i32);
    nc::wrefresh(self.win);
  }

  pub fn render(&self) {
    for (i, ch) in self.buf.char_indices() {
      nc::mvwaddch(self.win, 0, (i * 2) as i32, ch as u32);
    }

    for i in (self.buf.len()..self.max) {
      nc::mvwaddch(self.win, 0, (i * 2) as i32, '_' as u32);
    }

    nc::wrefresh(self.win);

    self.render_cur();
  }

  pub fn del_left(&mut self) {
    if !self.buf.is_empty() {
      self.cur = self.cur - 1;
      self.buf.remove(self.cur);
      self.render();
    }
  }

  pub fn del_right(&mut self) {
    if !self.buf.is_empty() && self.cur < self.buf.len() {
      self.buf.remove(self.cur);
      self.render();
    }
  }

  pub fn clear(&mut self) {
    self.buf.clear();
    self.cur = 0;
    self.render();
  }

  pub fn put(&mut self, s: &str) -> bool {
    return if s.len() + self.buf.len() <= self.max {
      self.buf.insert_str(self.cur, s);
      self.cur = self.cur + s.len();
      self.render();

      true
    } else {
      false
    };
  }

  pub fn move_to(&mut self, to: usize) {
    self.cur = cmp::max(0, cmp::min(self.buf.len(), to));
    self.render_cur();
  }

  pub fn move_by(&mut self, by: isize) {
    let cur = self.cur as isize;
    self.move_to((cur as isize + by) as usize);
  }

  pub fn left(&mut self) {
    self.move_by(-1);
  }

  pub fn right(&mut self) {
    self.move_by(1);
  }

  pub fn home(&mut self) {
    self.move_to(0);
  }

  pub fn end(&mut self) {
    let pos = self.buf.len();
    self.move_to(pos);
  }
}
