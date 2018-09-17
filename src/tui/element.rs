use std::{cell::RefCell, rc::Rc};
use tui::prelude_internal::*;

pub struct ElementCoreData {
  desired_size: MeasureSize,
}

impl Default for ElementCoreData {
  fn default() -> Self {
    Self {
      desired_size: Default::default(),
    }
  }
}

pub trait Element {
  fn desired_size(&self) -> MeasureSize;

  fn measure(&mut self, MeasureSize);

  fn arrange(&mut self, Rect);

  fn render(&mut self);

  fn render_cur(&mut self);
}

impl<T> Element for T
where
  T: ElementCore,
{
  #[inline]
  fn desired_size(&self) -> MeasureSize {
    self.get_coredata().desired_size
  }

  fn measure(&mut self, space: MeasureSize) {
    let val = self.measure_impl(space);
    self.get_coredata_mut().desired_size = val;
  }

  fn arrange(&mut self, space: Rect) {
    self.arrange_impl(space);
  }

  fn render(&mut self) {
    self.render_impl();
  }

  #[inline]
  fn render_cur(&mut self) {
    self.render_cur_impl();
  }
}

pub type ElemWrapper<T> = Rc<RefCell<T>>;

pub type ElemRef<'a> = Rc<RefCell<Element + 'a>>;

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
