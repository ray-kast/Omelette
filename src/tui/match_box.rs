extern crate ncurses as nc;

use tui::prelude_internal::*;
use word_list::WordlistForm;

pub struct MatchBox<'a> {
  coredata: ElementCoreData,
  win: nc::WINDOW,
  form: &'a WordlistForm,
  revealed: bool,
}

impl<'a> MatchBox<'a> {
  pub fn new(form: &'a WordlistForm) -> Self {
    Self {
      coredata: Default::default(),
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
  fn get_coredata(&self) -> &ElementCoreData {
    &self.coredata
  }

  fn get_coredata_mut(&mut self) -> &mut ElementCoreData {
    &mut self.coredata
  }

  fn measure_impl(&mut self, space: MeasureSize) -> MeasureSize {
    MeasureSize {
      w: Some(self.displayed_str().len() as i32),
      h: Some(1),
    }
  }

  fn arrange_impl(&mut self, space: Rect) {
    // TODO: these shouldn't be in arrange
    nc::wresize(self.win, 1, self.form.full.len() as i32);
    nc::mvwin(self.win, space.pos.y, space.pos.x);
  }

  fn render_impl(&mut self) {
    nc::mvwaddstr(self.win, 0, 0, self.displayed_str());

    nc::wrefresh(self.win);
  }
}
