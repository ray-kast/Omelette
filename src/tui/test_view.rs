use tui::prelude_internal::*;

pub struct TestView {
  desired_size: Size,
  child: ElemRef,
}

impl TestView {
  pub fn new(child: ElemRef) -> Self {
    Self {
      desired_size: Size { w: 0, h: 0 },
      child,
    }
  }
}

impl ElementCore for TestView {
  fn set_desired_size(&mut self, val: Size) {
    self.desired_size = val;
  }

  fn desired_size(&self) -> Size {
    self.desired_size
  }

  fn measure_impl(&self, space: Size) -> Size {
    let mut child = self.child.borrow_mut();
    child.measure(space);
    space
  }

  fn arrange_impl(&mut self, space: Rect) {
    let mut child = self.child.borrow_mut();

    let child_size = child.desired_size();

    let child_space = Rect {
      pos: Point {
        x: (space.size.w - child_size.w) / 2,
        y: (space.size.h - child_size.h) / 2,
      },
      size: child.desired_size(),
    };

    child.arrange(child_space);
  }

  fn render_impl(&self) {
    let child = self.child.borrow_mut();
    child.render();
  }
}
