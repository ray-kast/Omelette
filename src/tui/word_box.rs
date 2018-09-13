extern crate ncurses as nc;

use std::cmp;
use tui::prelude_internal::*;

pub struct WordBox {
  coredata: ElementCoreData,
  win: nc::WINDOW,
  cur: usize,
  max: usize,
  pub buf: String,
}

impl WordBox {
  pub fn new(max: usize) -> Self {
    Self {
      coredata: Default::default(),
      win: nc::newwin(1, 1, 0, 0),
      cur: 0,
      max,
      buf: String::new(),
    }
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
  fn get_coredata(&self) -> &ElementCoreData {
    &self.coredata
  }

  fn get_coredata_mut(&mut self) -> &mut ElementCoreData {
    &mut self.coredata
  }

  fn measure_impl(&mut self, _: MeasureSize) -> MeasureSize {
    MeasureSize {
      w: Some(self.max as i32 * 2 + 1),
      h: Some(1),
    }
  }

  fn arrange_impl(&mut self, space: Rect) {
    // TODO: these shouldn't be in arrange
    nc::wresize(self.win, 1, self.max as i32 * 2 - 1);
    nc::mvwin(self.win, space.pos.y, space.pos.x);
  }

  fn render_impl(&mut self) {
    for (i, ch) in self.buf.char_indices() {
      nc::mvwaddch(self.win, 0, (i * 2) as i32, ch as u32);
    }

    for i in self.buf.len()..self.max {
      nc::mvwaddch(self.win, 0, (i * 2) as i32, '_' as u32);
    }

    nc::wrefresh(self.win);

    self.render_cur();
  }

  fn render_cur_impl(&mut self) {
    nc::wmove(self.win, 0, (self.cur * 2) as i32);
    nc::wrefresh(self.win);
  }
}
