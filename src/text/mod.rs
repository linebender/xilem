// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Editing and displaying text.

mod attribute;
mod backspace;
mod editable_text;
mod font_descriptor;

#[deprecated(since = "0.8.0", note = "use types from druid::text module instead")]
#[doc(hidden)]
pub mod format;
// a hack to let us deprecate the format module; we can remove this when we make
// format private
#[path = "format.rs"]
mod format_priv;
mod input_component;
mod input_methods;
mod layout;
mod movement;
mod rich_text;
mod storage;

pub use druid_shell::text::{
    Action as TextAction, Affinity, Direction, Event as ImeInvalidation, InputHandler, Movement,
    Selection, VerticalMovement, WritingDirection,
};
pub use input_component::{EditSession, TextComponent};
pub use input_methods::ImeHandlerRef;
pub(crate) use input_methods::TextFieldRegistration;
pub use rich_text::{AttributesAdder, RichText, RichTextBuilder};
pub use storage::{ArcStr, TextStorage};

pub use self::attribute::{Attribute, AttributeSpans, Link};
pub use self::backspace::offset_for_delete_backwards;
pub use self::editable_text::{EditableText, EditableTextCursor, StringCursor};
pub use self::font_descriptor::FontDescriptor;
pub use self::format_priv::{Formatter, ParseFormatter, Validation, ValidationError};
pub use self::layout::{LayoutMetrics, TextLayout};
pub use self::movement::movement;
pub use crate::piet::{FontFamily, FontStyle, FontWeight, TextAlignment};
