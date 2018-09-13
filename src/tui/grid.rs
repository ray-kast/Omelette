use std::{
  cmp,
  collections::{HashMap, HashSet},
};
use tui::prelude_internal::*;

pub enum GridLength {
  Static(i32),
  Dynamic(f32),
  Content,
}

pub struct Grid<'a> {
  coredata: ElementCoreData,
  control_cells: HashMap<(usize, usize), HashSet<usize>>,
  row_sizes: Vec<i32>,
  col_sizes: Vec<i32>,
  children: Vec<ElemRef<'a>>,
  rows: Vec<GridLength>,
  cols: Vec<GridLength>,
}

impl<'a> Grid<'a> {
  pub fn new<IC, IR, IK>(children: IC, rows: IR, cols: IK) -> Self
  where
    IC: IntoIterator<Item = (ElemRef<'a>, (usize, usize))>,
    IR: IntoIterator<Item = GridLength>,
    IK: IntoIterator<Item = GridLength>,
  {
    let mut control_cells: HashMap<(usize, usize), HashSet<usize>> =
      HashMap::new();

    let children = children
      .into_iter()
      .enumerate()
      .map(|(i, (child, cell))| {
        use std::collections::hash_map::Entry::*;

        match control_cells.entry(cell) {
          Vacant(v) => {
            v.insert(HashSet::new()).insert(i);
          }
          Occupied(mut o) => {
            o.get_mut().insert(i);
          }
        }

        child
      })
      .collect();

    let rows: Vec<_> = rows.into_iter().collect();
    let cols: Vec<_> = cols.into_iter().collect();

    let mut row_sizes = Vec::with_capacity(rows.len());
    let mut col_sizes = Vec::with_capacity(cols.len());

    for _ in 0..rows.len() {
      row_sizes.push(0);
    }
    for _ in 0..cols.len() {
      col_sizes.push(0);
    }

    Self {
      coredata: Default::default(),
      control_cells,
      row_sizes,
      col_sizes,
      children,
      rows,
      cols,
    }
  }
}

impl<'a> ElementCore for Grid<'a> {
  fn get_coredata(&self) -> &ElementCoreData {
    &self.coredata
  }

  fn get_coredata_mut(&mut self) -> &mut ElementCoreData {
    &mut self.coredata
  }

  fn measure_impl(&mut self, space: MeasureSize) -> MeasureSize {
    use std::collections::hash_map::Entry::*;
    use GridLength::*;

    let mut static_cells: Vec<(usize, usize)> = Vec::new();
    let mut dynamic_cells: Vec<(usize, usize)> = Vec::new(); // TODO: is this necessary?
    let mut content_cells: Vec<(usize, usize)> = Vec::new();

    let mut row_sizes: HashMap<usize, i32> = HashMap::new();
    let mut col_sizes: HashMap<usize, i32> = HashMap::new();

    for (i, row) in self.rows.iter().enumerate() {
      for (j, col) in self.cols.iter().enumerate() {
        let cell = (i, j);

        match row {
          Static(_) => match col {
            Static(_) => &mut static_cells,
            Dynamic(_) => &mut dynamic_cells,
            Content => &mut content_cells,
          },
          Dynamic(_) => match col {
            Static(_) => &mut dynamic_cells,
            Dynamic(_) => &mut dynamic_cells,
            Content => &mut content_cells,
          },
          Content => match col {
            Static(_) => &mut content_cells,
            Dynamic(_) => &mut content_cells,
            Content => &mut content_cells,
          },
        }.push(cell);
      }
    }

    for (i, j) in static_cells {
      match self.cols[j] {
        Static(w) => {
          col_sizes.insert(j, w);
        }
        _ => unreachable!(),
      }

      match self.rows[i] {
        Static(h) => {
          row_sizes.insert(i, h);
        }
        _ => unreachable!(),
      }
    }

    for (i, j) in content_cells {
      let cell = (i, j);

      match self.rows[i] {
        Content => match self.control_cells.get(&cell) {
          Some(c) => for k in c {
            let mut child = self.children[*k].borrow_mut();

            child.measure(MeasureSize {
              w: None,
              h: Some(0),
            });

            let size = child.desired_size().h.unwrap_or(0);

            match row_sizes.entry(i) {
              Vacant(v) => {
                v.insert(size);
              }
              Occupied(mut o) => {
                let v = o.get_mut();
                *v = cmp::max(*v, size);
              }
            }
          },
          None => (),
        },
        _ => match self.cols[j] {
          Content => match self.control_cells.get(&cell) {
            Some(c) => for k in c {
              let mut child = self.children[*k].borrow_mut();

              child.measure(MeasureSize {
                w: Some(0),
                h: None,
              });

              let size = child.desired_size().w.unwrap_or(0);

              match col_sizes.entry(j) {
                Vacant(v) => {
                  v.insert(size);
                }
                Occupied(mut o) => {
                  let v = o.get_mut();
                  *v = cmp::max(*v, size);
                }
              }
            },
            None => (),
          },
          _ => unreachable!(),
        },
      }
    }

    let mut used_size = Size { w: 0, h: 0 };
    let mut total_weight_r = 0.0;
    let mut total_weight_c = 0.0;

    for (_, size) in &row_sizes {
      used_size.h = used_size.h + size;
    }

    for (_, size) in &col_sizes {
      used_size.w = used_size.w + size;
    }

    let free_space = Size {
      w: space.w.map_or(0, |w| cmp::max(0, w - used_size.w)),
      h: space.h.map_or(0, |h| cmp::max(0, h - used_size.h)),
    };

    for row in &self.rows {
      match row {
        Dynamic(w) => total_weight_r = total_weight_r + w,
        _ => (),
      }
    }

    for col in &self.cols {
      match col {
        Dynamic(w) => total_weight_c = total_weight_c + w,
        _ => (),
      }
    }

    // TODO: this does not handle rounding errors *at all*
    for (i, row) in self.rows.iter().enumerate() {
      match row {
        Dynamic(h) => {
          row_sizes.insert(
            i,
            (free_space.h as f32 * h / total_weight_r).round() as i32,
          );
        }
        _ => (),
      }
    }

    for (i, col) in self.cols.iter().enumerate() {
      match col {
        Dynamic(w) => {
          col_sizes.insert(
            i,
            (free_space.w as f32 * w / total_weight_c).round() as i32,
          );
        }
        _ => (),
      }
    }

    for i in 0..self.rows.len() {
      self.row_sizes[i] = row_sizes[&i];
    }

    for i in 0..self.cols.len() {
      self.col_sizes[i] = col_sizes[&i];
    }

    for i in 0..self.rows.len() {
      for j in 0..self.cols.len() {
        let cell = (i, j);

        match self.control_cells.get(&cell) {
          Some(c) => for k in c {
            let mut child = self.children[*k].borrow_mut();

            child.measure(MeasureSize {
              w: col_sizes.get(&j).map(|v| *v),
              h: row_sizes.get(&j).map(|v| *v),
            });
          },
          None => (),
        }
      }
    }

    MeasureSize {
      w: Some(used_size.w),
      h: Some(used_size.h),
    }
  }

  // NB: ignoring the size because it would probably require re-measuring
  fn arrange_impl(&mut self, space: Rect) {
    let mut row_pos: i32 = 0;
    let mut col_pos: Vec<i32> = Vec::new();

    {
      let mut pos: i32 = 0;

      for i in 0..self.cols.len() {
        col_pos.push(pos);
        pos = pos + self.col_sizes[i];
      }
    }

    for i in 0..self.rows.len() {
      let row_size = self.row_sizes[i];

      for j in 0..self.cols.len() {
        let col_size = self.col_sizes[j];
        let cell = (i, j);

        match self.control_cells.get(&cell) {
          Some(c) => for k in c {
            let mut child = self.children[*k].borrow_mut();

            child.arrange(Rect {
              pos: Point {
                x: space.pos.x + col_pos[j],
                y: space.pos.y + row_pos,
              },
              size: Size {
                w: col_size,
                h: row_size,
              },
            })
          },
          None => (),
        }
      }

      row_pos = row_pos + row_size;
    }
  }

  fn render_impl(&mut self) {
    for child in &self.children {
      let mut child = child.borrow_mut();

      child.render();
    }
  }
}
