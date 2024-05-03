//! Support for text display and rendering
//!
//! There are three kinds of text commonly needed:
//!  1) Entirely display text (e.g. a button)
//!  2) Selectable text (e.g. a paragraph of content)
//!  3) Editable text (e.g. a search bar)
//!
//! All of these have the same set of global styling options, and can contain rich text

mod edit;
mod layout;
// mod movement;
mod selection;
mod store;

pub use layout::{LayoutMetrics, TextBrush, TextLayout};
pub use selection::TextWithSelection;
pub use selection::{len_utf8_from_first_byte, EditableTextCursor, Selectable, StringCursor};
pub use store::{Link, TextStorage};
