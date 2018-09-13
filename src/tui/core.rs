#[derive(Clone, Copy)]
pub struct Point {
  pub x: i32,
  pub y: i32,
}

#[derive(Clone, Copy)]
pub struct Size {
  pub w: i32,
  pub h: i32,
}

#[derive(Clone, Copy)]
pub struct MeasureSize {
  pub w: Option<i32>,
  pub h: Option<i32>,
}

#[derive(Clone, Copy)]
pub struct Rect {
  pub pos: Point,
  pub size: Size,
}

impl Default for MeasureSize {
  fn default() -> Self {
    Self { w: None, h: None }
  }
}
