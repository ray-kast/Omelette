use std::cmp;
use tui::prelude_internal::*;

pub enum WrapMode {
  Rows,
  Cols,
}

pub enum WrapAlign {
  Begin,
  Middle,
  End,
  Stretch,
}

pub struct WrapBox<'a> {
  coredata: ElementCoreData,
  lines: Vec<(Size, usize)>,
  children: Vec<ElemRef<'a>>,
  mode: WrapMode,
  align: WrapAlign,
  line_sep: i32,
}

impl<'a> WrapBox<'a> {
  pub fn new<IC>(
    children: IC,
    mode: WrapMode,
    align: WrapAlign,
    line_sep: i32,
  ) -> Self
  where
    IC: IntoIterator<Item = ElemRef<'a>>,
  {
    Self {
      coredata: Default::default(),
      lines: Vec::new(),
      children: children.into_iter().collect(),
      mode,
      align,
      line_sep,
    }
  }
}

impl<'a> ElementCore for WrapBox<'a> {
  fn get_coredata(&self) -> &ElementCoreData {
    &self.coredata
  }

  fn get_coredata_mut(&mut self) -> &mut ElementCoreData {
    &mut self.coredata
  }

  fn measure_impl(&mut self, space: MeasureSize) -> MeasureSize {
    use WrapAlign::*;
    use WrapMode::*;

    self.lines.clear();

    match self.mode {
      Rows => {
        let mut row_width = 0;
        let mut row_max_height = 0;
        let mut max_width = 0;
        let mut height = 0;
        let mut width_left = space.w;
        let mut count = 0;

        for child in &self.children {
          let mut child = child.borrow_mut();

          child.measure(MeasureSize {
            w: Some(0),
            h: Some(0),
          });

          let child_size = child.desired_size();
          let child_w = child_size.w.unwrap_or(0);

          if width_left.map_or(false, |w| child_w > w) {
            self.lines.push((
              Size {
                w: row_width,
                h: row_max_height,
              },
              count,
            ));

            max_width = cmp::max(max_width, row_width);
            height = height + row_max_height + self.line_sep;

            row_width = 0;
            row_max_height = 0;
            width_left = space.w;
            count = 0;
          }

          row_width = row_width + child_w;
          width_left = width_left.map(|w| w - child_w);
          row_max_height = cmp::max(row_max_height, child_size.h.unwrap_or(0));
          count = count + 1;
        }

        if count > 0 {
          self.lines.push((
            Size {
              w: row_width,
              h: row_max_height,
            },
            count,
          ));
        }

        // TODO: re-measure children if align == Stretch

        MeasureSize {
          w: Some(max_width),
          h: Some(cmp::max(0, height - self.line_sep)),
        }
      }
      Cols => {
        let mut col_height = 0;
        let mut col_max_width = 0;
        let mut max_height = 0;
        let mut width = 0;
        let mut height_left = space.h;
        let mut count = 0;

        for child in &self.children {
          let mut child = child.borrow_mut();

          child.measure(MeasureSize {
            w: Some(0),
            h: Some(0),
          });

          let child_size = child.desired_size();
          let child_h = child_size.h.unwrap_or(0);

          if height_left.map_or(false, |h| child_h > h) {
            self.lines.push((
              Size {
                w: col_max_width,
                h: col_height,
              },
              count,
            ));

            max_height = cmp::max(max_height, col_height);
            width = width + col_max_width + self.line_sep;

            col_height = 0;
            col_max_width = 0;
            height_left = space.h;
            count = 0;
          }

          col_height = col_height + child_h;
          height_left = height_left.map(|h| h - child_h);
          col_max_width = cmp::max(col_max_width, child_size.w.unwrap_or(0));
          count = count + 1;
        }

        if count > 0 {
          self.lines.push((
            Size {
              w: col_max_width,
              h: col_height,
            },
            count,
          ));
        }

        // TODO: re-measure children if align == Stretch

        MeasureSize {
          w: Some(cmp::max(0, width - self.line_sep)),
          h: Some(max_height),
        }
      }
    }
  }

  fn arrange_impl(&mut self, space: Rect) {
    use WrapAlign::*;
    use WrapMode::*;

    match self.mode {
      Rows => {
        let mut i: usize = 0;
        let mut row = (Size { w: 0, h: 0 }, 0);
        let mut pos = Point { x: 0, y: -self.line_sep };

        for child in &self.children {
          if row.1 == 0 {
            pos.y = pos.y + row.0.h + self.line_sep; // TODO
            row = self.lines[i];
            i = i + 1;
            pos.x = 0; // TODO: pay attention to alignment when doing this
          }

          let mut child = child.borrow_mut();

          let child_w = child.desired_size().w.unwrap_or(0);

          child.arrange(Rect {
            pos: Point {
              x: pos.x + space.pos.x,
              y: pos.y + space.pos.y,
            },
            size: Size {
              w: child_w,
              h: row.0.h,
            },
          });

          pos.x = pos.x + child_w;
          row.1 = row.1 - 1;
        }
      }
      Cols => {
        let mut i: usize = 0;
        let mut col = (Size { w: 0, h: 0 }, 0);
        let mut pos = Point { x: -self.line_sep, y: 0 };

        for child in &self.children {
          if col.1 == 0 {
            pos.x = pos.x + col.0.w + self.line_sep; // TODO
            col = self.lines[i];
            i = i + 1;
            pos.y = 0; // TODO: pay attention to alignment when doing this
          }

          let mut child = child.borrow_mut();

          let child_h = child.desired_size().h.unwrap_or(0);

          child.arrange(Rect {
            pos: Point {
              x: pos.x + space.pos.x,
              y: pos.y + space.pos.y,
            },
            size: Size {
              w: col.0.w,
              h: child_h,
            },
          });

          pos.y = pos.y + child_h;
          col.1 = col.1 - 1;
        }
      }
    }
  }

  fn render_impl(&mut self) {
    for child in &self.children {
      child.borrow_mut().render();
    }
  }
}
