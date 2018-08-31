use tui::prelude_internal::*;

pub struct TestView<'a> {
  desired_size: Size,
  word_box: ElemRef<'a>,
  match_box: ElemRef<'a>,
}

impl<'a> TestView<'a> {
  pub fn new(word_box: ElemRef<'a>, match_box: ElemRef<'a>) -> Self {
    Self {
      desired_size: Size { w: 0, h: 0 },
      word_box,
      match_box,
    }
  }
}

impl<'a> ElementCore for TestView<'a> {
  fn set_desired_size(&mut self, val: Size) {
    self.desired_size = val;
  }

  fn desired_size(&self) -> Size {
    self.desired_size
  }

  fn measure_impl(&self, space: Size) -> Size {
    {
      let mut match_box = self.match_box.borrow_mut();
      match_box.measure(space);
    }

    {
      let mut word_box = self.word_box.borrow_mut();
      word_box.measure(space);
    }

    space
  }

  fn arrange_impl(&mut self, space: Rect) {
    {
      let mut match_box = self.match_box.borrow_mut();

      let size = match_box.desired_size();

      match_box.arrange(Rect {
        pos: Point { x: 1, y: 1 },
        size,
      });
    }

    {
      let mut word_box = self.word_box.borrow_mut();

      let size = word_box.desired_size();

      word_box.arrange(Rect {
        pos: Point {
          x: 1,
          y: self.desired_size.h - 2,
        },
        size,
      });
    }
  }

  fn render_impl(&self) {
    {
      let match_box = self.match_box.borrow_mut();
      match_box.render();
    }

    {
      let word_box = self.word_box.borrow_mut();
      word_box.render();
    }
  }
}
