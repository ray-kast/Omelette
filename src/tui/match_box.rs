extern crate ncurses as nc;

use tui::prelude_internal::*;
use word_list::WordlistForm;

pub struct MatchBox<'a> {
  desired_size: Size,
  win: nc::WINDOW,
  form: &'a WordlistForm,
  revealed: bool,
}

impl<'a> MatchBox<'a> {
  pub fn new(form: &'a WordlistForm) -> Self {
    Self {
      desired_size: Size { w: 0, h: 0 },
      win: nc::newwin(1, 1, 0, 0),
      form,
      revealed: false,
    }
  }

  pub fn form(&self) -> &WordlistForm {
    &self.form
  }

  pub fn revealed(&self) -> bool {
    self.revealed
  }

  pub fn set_revealed(&mut self, val: bool) {
    self.revealed = val;
    self.render();
  }

  fn displayed_str(&self) -> &str {
    if self.revealed {
      &self.form.full
    } else {
      &self.form.blanked
    }
  }
}

impl<'a> ElementCore for MatchBox<'a> {
  fn set_desired_size(&mut self, val: Size) {
    self.desired_size = val;
  }

  fn desired_size(&self) -> Size {
    self.desired_size
  }

  fn measure_impl(&self, space: Size) -> Size {
    Size {
      w: self.displayed_str().len() as i32,
      h: 1,
    }
  }

  fn arrange_impl(&mut self, space: Rect) {
    // TODO: these shouldn't be in arrange
    nc::wresize(self.win, 1, self.form.full.len() as i32);
    nc::mvwin(self.win, space.pos.x, space.pos.y);
  }

  fn render_impl(&self) {
    nc::mvwaddstr(self.win, 0, 0, self.displayed_str());

    nc::wrefresh(self.win);
  }
}
