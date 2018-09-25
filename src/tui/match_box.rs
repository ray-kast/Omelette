use nc;
use tui::prelude_internal::*;
use word_list::WordlistForm;

pub enum MatchBoxStyle {
  Normal,
  Reveal,
  Highlight,
}

pub struct MatchBox {
  coredata: ElementCoreData,
  win: nc::WINDOW,
  form: WordlistForm,
  revealed: bool,
  style: MatchBoxStyle,
  reveal_pair: i32,
  hl_pair: i32,
}

impl MatchBox {
  pub fn new(form: WordlistForm, reveal_pair: i32, hl_pair: i32) -> Self {
    Self {
      coredata: Default::default(),
      win: nc::newwin(1, 1, 0, 0),
      form,
      revealed: false,
      style: MatchBoxStyle::Normal,
      reveal_pair,
      hl_pair,
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

  pub fn set_style(&mut self, val: MatchBoxStyle) {
    if self.revealed {
      self.style = val;
      self.render();
    }
  }

  fn displayed_str(&self) -> &str {
    if self.revealed {
      &self.form.full
    } else {
      &self.form.blanked
    }
  }
}

impl ElementCore for MatchBox {
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
    nc::wresize(self.win, 1, self.form.full.len() as i32);
    nc::mvwin(self.win, space.pos.y, space.pos.x);
  }

  fn render_impl(&mut self) {
    use MatchBoxStyle::*;

    let pair = match self.style {
      Normal => None,
      Reveal => Some(self.reveal_pair),
      Highlight => Some(self.hl_pair),
    }.map(|p| nc::COLOR_PAIR(p as i16));

    if let Some(pair) = pair {
      nc::wattr_on(self.win, pair);
    }

    nc::mvwaddstr(self.win, 0, 0, self.displayed_str());

    if let Some(pair) = pair {
      nc::wattr_off(self.win, pair);
    }

    nc::wrefresh(self.win);
  }
}
