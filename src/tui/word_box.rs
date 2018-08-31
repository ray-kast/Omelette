extern crate ncurses as nc;

use std::cmp;
use tui::prelude_internal::*;

pub struct WordBox {
  desired_size: Size,
  win: nc::WINDOW,
  cur: usize,
  max: usize,
  pub buf: String,
}

impl WordBox {
  pub fn new(max: usize) -> Self {
    Self {
      desired_size: Size { w: 0, h: 0 },
      win: nc::newwin(1, 1, 0, 0),
      cur: 0,
      max,
      buf: String::new(),
    }
  }

  pub fn render_cur(&self) {
    nc::wmove(self.win, 0, (self.cur * 2) as i32);
    nc::wrefresh(self.win);
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
    if s.len() + self.buf.len() <= self.max {
      self.buf.insert_str(self.cur, s);
      self.cur = self.cur + s.len();
      self.render();

      true
    } else {
      false
    }
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

impl ElementCore for WordBox {
  fn set_desired_size(&mut self, val: Size) {
    self.desired_size = val;
  }

  fn desired_size(&self) -> Size {
    self.desired_size
  }

  fn measure_impl(&self, _: Size) -> Size {
    Size {
      w: self.max as i32 * 2 + 1,
      h: 1,
    }
  }

  fn arrange_impl(&mut self, space: Rect) {
    // TODO: these shouldn't be in arrange
    nc::wresize(self.win, 1, self.max as i32 * 2 - 1);
    nc::mvwin(self.win, space.pos.y, space.pos.x);
  }

  fn render_impl(&self) {
    for (i, ch) in self.buf.char_indices() {
      nc::mvwaddch(self.win, 0, (i * 2) as i32, ch as u32);
    }

    for i in self.buf.len()..self.max {
      nc::mvwaddch(self.win, 0, (i * 2) as i32, '_' as u32);
    }

    nc::wrefresh(self.win);

    self.render_cur();
  }
}
