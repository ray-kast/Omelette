pub mod core;
pub mod element;
pub mod internal;

mod match_box;
mod test_view;
mod ui_root;
mod word_box;

pub mod prelude_internal {
  pub use super::{core::*, element::*, internal::*};
}

pub mod controls {
  pub use super::{match_box::*, test_view::*, ui_root::*, word_box::*};
}
