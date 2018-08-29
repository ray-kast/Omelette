use tui::core::*;

pub trait ElementCore {
  fn set_desired_size(&mut self, Size);

  fn desired_size(&self) -> Size;

  fn measure_impl(&self, Size) -> Size;

  fn arrange_impl(&mut self, Rect);

  fn render_impl(&self);
}
