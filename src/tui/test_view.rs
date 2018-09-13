use tui::{element as el, grid::*, prelude_internal::*};

pub struct TestView<'a> {
  coredata: ElementCoreData,
  grid: ElemRef<'a>,
  word_box: ElemRef<'a>,
}

impl<'a> TestView<'a> {
  pub fn new(word_box: ElemRef<'a>, match_box: ElemRef<'a>) -> Self {
    Self {
      coredata: Default::default(),
      grid: el::wrap(Grid::new(
        vec![(word_box.clone(), (1, 0)), (match_box, (0, 0))],
        vec![GridLength::Dynamic(1.0), GridLength::Content],
        vec![GridLength::Dynamic(1.0)],
      )),
      word_box,
    }
  }
}

impl<'a> ElementCore for TestView<'a> {
  fn get_coredata(&self) -> &ElementCoreData {
    &self.coredata
  }

  fn get_coredata_mut(&mut self) -> &mut ElementCoreData {
    &mut self.coredata
  }

  fn measure_impl(&mut self, space: MeasureSize) -> MeasureSize {
    let mut grid = self.grid.borrow_mut();
    grid.measure(space);

    grid.desired_size()
  }

  fn arrange_impl(&mut self, space: Rect) {
    let mut grid = self.grid.borrow_mut();
    grid.arrange(space);
  }

  fn render_impl(&mut self) {
    let mut grid = self.grid.borrow_mut();
    grid.render();

    let mut word_box = self.word_box.borrow_mut();
    word_box.render_cur();
  }
}
