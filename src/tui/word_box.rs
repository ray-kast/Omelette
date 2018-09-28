use nc;
use rand::{self, prelude::*};
use std::cmp;
use tui::prelude_internal::*;

pub struct WordBox {
  coredata: ElementCoreData,
  win: nc::WINDOW,
  cur: usize,
  buf: String,
  ghost_buf: String,
  bad: bool,
  auto_sort: bool,
  key: String,
  ghost_pair: i32,
  bad_ghost_pair: i32,
  auto_ghost_pair: i32,
}

impl WordBox {
  pub fn new(
    key: String,
    ghost_pair: i32,
    bad_ghost_pair: i32,
    auto_ghost_pair: i32,
  ) -> Self {
    let ghost_buf = key.clone();

    Self {
      coredata: Default::default(),
      win: nc::newwin(1, 1, 0, 0),
      cur: 0,
      buf: String::new(),
      ghost_buf,
      bad: false,
      auto_sort: false,
      key,
      ghost_pair,
      bad_ghost_pair,
      auto_ghost_pair,
    }
  }

  pub fn buf(&self) -> &String {
    &self.buf
  }

  pub fn set_bad(&mut self, val: bool) {
    if self.bad == val {
      return;
    }

    self.bad = val;

    self.render();
  }

  pub fn auto_sort(&self) -> bool {
    self.auto_sort
  }

  pub fn set_auto_sort(&mut self, val: bool) {
    self.auto_sort = val;

    self.fix_ghost();
    self.render();
  }

  fn fix_ghost(&mut self) {
    if self.auto_sort {
      let mut chars: Vec<_> = self.ghost_buf.chars().collect();
      chars.sort();
      self.ghost_buf = chars.into_iter().collect();
    }
  }

  fn remove(&mut self, at: usize) {
    self.ghost_buf.insert(0, self.buf.remove(at));
    self.fix_ghost();
  }

  fn del_empty(&mut self) {
    if self.auto_sort {
      self.bad = false;
      self.render();
    } else if self.bad {
      self.auto_sort = false;
      self.render();
    }
  }

  pub fn del_left(&mut self) {
    if self.buf.is_empty() {
      self.del_empty();
    } else {
      if self.cur > 0 {
        self.cur = self.cur - 1;
        let cur = self.cur;
        self.remove(cur);
        self.render();
      } else if self.buf.len() == 1 {
        self.remove(0);
        self.render();
      }
    }

  }

  pub fn del_right(&mut self) {
    if self.buf.is_empty() {
      self.del_empty();
    } else {
      if self.cur < self.buf.len() {
        let cur = self.cur;
        self.remove(cur);
        self.render();
      }
    }
  }

  pub fn clear(&mut self) {
    if self.buf.is_empty() { // TODO: move this block elsewhere probably
      self.del_empty();
    }
    self.ghost_buf.insert_str(0, &self.buf);
    self.buf.clear();
    self.cur = 0;
    self.fix_ghost();
    self.render();
  }

  pub fn put(&mut self, s: &str) {
    let mut dirty = false;

    for c in s.chars() {
      if self.buf.len() >= self.key.len() {
        break;
      }

      match self.ghost_buf.find(c) {
        Some(i) => {
          dirty = true;
          self.buf.insert(self.cur, c);
          self.cur = self.cur + 1;
          self.ghost_buf.remove(i);
        }
        None => {}
      }
    }

    if dirty {
      self.fix_ghost();
      self.render();
    }
  }

  pub fn move_to(&mut self, to: usize) {
    self.cur = cmp::max(0, cmp::min(self.buf.len(), to));
    self.render_cur();
  }

  pub fn move_by(&mut self, by: isize) {
    let cur = self.cur as isize;
    self.move_to(cmp::max(0, cur as isize + by) as usize);
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

  pub fn shuffle(&mut self) {
    self.auto_sort = false;

    let mut chars: Vec<_> = self.ghost_buf.chars().collect();

    self.ghost_buf.clear();

    let mut i: usize = 0;

    while !chars.is_empty() {
      let j = rand::thread_rng().gen_range(0, chars.len());
      self.ghost_buf.push(chars.remove(j));
      i = i + 1;
    }

    self.render();
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
      w: Some(self.key.len() as i32 * 2 + 1),
      h: Some(1),
    }
  }

  fn arrange_impl(&mut self, space: Rect) {
    nc::wresize(self.win, 1, self.key.len() as i32 * 2 - 1);
    nc::mvwin(self.win, space.pos.y, space.pos.x);
  }

  fn render_impl(&mut self) {
    for (i, ch) in self.buf.char_indices() {
      nc::mvwaddch(self.win, 0, (i * 2) as i32, ch as u32);
    }

    let pair = nc::COLOR_PAIR(if self.auto_sort {
      self.auto_ghost_pair
    } else {
      if self.bad {
        self.bad_ghost_pair
      } else {
        self.ghost_pair
      }
    } as i16);

    nc::wattr_on(self.win, pair);

    let buf_len = self.buf.len();

    for (i, ch) in self.ghost_buf.char_indices() {
      nc::mvwaddch(self.win, 0, ((i + buf_len) * 2) as i32, ch as u32);
    }

    nc::wattr_off(self.win, pair);

    nc::wrefresh(self.win);

    self.render_cur();
  }

  fn render_cur_impl(&mut self) {
    nc::wmove(self.win, 0, (self.cur * 2) as i32);
    nc::wrefresh(self.win);
  }
}
