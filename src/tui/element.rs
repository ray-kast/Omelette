use std::{cell::RefCell, rc::Rc};
use tui::prelude_internal::*;

pub trait Element {
  fn desired_size(&self) -> Size;

  fn measure(&mut self, Size);

  fn arrange(&mut self, Rect);

  fn render(&self);
}

impl<T: ElementCore> Element for T {
  fn desired_size(&self) -> Size {
    ElementCore::desired_size(self)
  }

  fn measure(&mut self, space: Size) {
    let val = self.measure_impl(space);
    self.set_desired_size(val);
  }

  fn arrange(&mut self, space: Rect) {
    self.arrange_impl(space);
  }

  fn render(&self) {
    self.render_impl();
  }
}

pub type ElemRef<'a> = Rc<RefCell<dyn Element + 'a>>;

pub fn wrap<T>(el: T) -> Rc<RefCell<T>>
where
  T: Element,
{
  Rc::new(RefCell::new(el))
}

pub fn add_ref<'a, T>(el: &Rc<RefCell<T>>) -> ElemRef
where
  T: Element + 'a,
{
  Rc::clone(el) as ElemRef
}
