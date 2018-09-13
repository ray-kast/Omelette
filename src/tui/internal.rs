use tui::{core::*, element::ElementCoreData};

pub trait ElementCore {
  fn get_coredata(&self) -> &ElementCoreData;

  fn get_coredata_mut(&mut self) -> &mut ElementCoreData;

  fn measure_impl(&mut self, MeasureSize) -> MeasureSize;

  fn arrange_impl(&mut self, Rect);

  fn render_impl(&mut self);

  fn render_cur_impl(&mut self) {}
}
