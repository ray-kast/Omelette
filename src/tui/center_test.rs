use tui::{core::*, element::*, internal::*};

pub struct CenterTest {
  desired_size: Size,
  pub child: ElemRef,
}

impl CenterTest {
  pub fn new(child: ElemRef) -> CenterTest {
    return CenterTest {
      desired_size: Size { w: 0, h: 0 },
      child: child,
    };
  }
}

impl ElementCore for CenterTest {
  fn set_desired_size(&mut self, val: Size) {
    self.desired_size = val;
  }

  fn desired_size(&self) -> Size {
    return self.desired_size;
  }

  fn measure_impl(&self, space: Size) -> Size {
    let mut child = self.child.borrow_mut();
    child.measure(space);
    return space;
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
