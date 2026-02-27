//
// Original code by: Mrmayman <navneetkrishna22@gmail.com>
// https://github.com/Mrmayman/frostmark
// I've only adapted it to work with libcosmic
//

#![allow(clippy::collapsible_if)]

mod renderer;
mod state;
mod structs;
mod style;
mod widgets;

pub use state::MarkState;
pub use structs::{ImageInfo, MarkWidget, RubyMode, UpdateMsg};
pub use style::Style;
