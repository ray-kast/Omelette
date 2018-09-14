use nc;
use std::{cmp, collections::HashMap};
use tui::prelude_internal::*;

pub struct WordBox {
  coredata: ElementCoreData,
  win: nc::WINDOW,
  cur: usize,
  max: usize,
  buf: String,
  avail: HashMap<char, usize>,
  remain: HashMap<char, usize>,
}

impl WordBox {
  pub fn new(max: usize, avail: HashMap<char, usize>) -> Self {
    let remain = avail.clone();

    Self {
      coredata: Default::default(),
      win: nc::newwin(1, 1, 0, 0),
      cur: 0,
      max,
      buf: String::new(),
      avail,
      remain,
    }
  }

  pub fn buf(&self) -> &String {
    &self.buf
  }

  fn remove(&mut self, at: usize) {
    use std::collections::hash_map::Entry::*;

    match self.remain.entry(self.buf.remove(self.cur)) {
      Vacant(v) => { v.insert(1); },
      Occupied(o) => {
        let mut v = o.into_mut();
        *v = *v + 1;
      }
    }
  }

  pub fn del_left(&mut self) {
    if !self.buf.is_empty() {
      self.cur = self.cur - 1;
      let cur = self.cur;
      self.remove(cur);
      self.render();
    }
  }

  pub fn del_right(&mut self) {
    if !self.buf.is_empty() && self.cur < self.buf.len() {
      let cur = self.cur;
      self.remove(cur);
      self.render();
    }
  }

  pub fn clear(&mut self) {
    self.buf.clear();
    self.cur = 0;
    self.remain = self.avail.clone();
    self.render();
  }

  pub fn put(&mut self, s: &str) {
    let mut dirty = false;

    for c in s.chars() {
      if self.buf.len() >= self.max {
        break;
      }

      use std::collections::hash_map::Entry::*;

      match self.remain.entry(c) {
        Vacant(_) => (),
        Occupied(o) => {
          let mut v = o.into_mut();

          if *v > 0 {
            self.buf.insert_str(self.cur, s);
            self.cur = self.cur + s.len();
            dirty = true;
            *v = *v - 1;
          }
        }
      }
    }

    if dirty {
      self.render();
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
