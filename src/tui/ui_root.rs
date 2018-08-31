extern crate ncurses as nc;

use tui::prelude_internal::*;

pub struct UiRoot {
  win: nc::WINDOW,
  child: ElemRef,
}

impl UiRoot {
  pub fn new(win: nc::WINDOW, child: ElemRef) -> Self {
    Self { win, child }
  }

  pub fn run(&mut self) {
    // TODO
  }

  pub fn resize(&self) {
    nc::wclear(self.win);
    nc::wrefresh(self.win);

    let mut size = Size { w: 0, h: 0 };
    nc::getmaxyx(self.win, &mut size.h, &mut size.w);

    let mut child = self.child.borrow_mut();

    child.measure(size);

    child.arrange(Rect {
      pos: Point { x: 0, y: 0 },
      size,
    });

    child.render();
  }
}
