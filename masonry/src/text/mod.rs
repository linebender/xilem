// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Support for text display and rendering
//!
//! There are three kinds of text commonly needed:
//!  1) Entirely display text (e.g. a button)
//!  2) Selectable text (e.g. a paragraph of content)
//!  3) Editable text (e.g. a search bar)
//!
//! All of these have the same set of global styling options, and can contain rich text

mod layout;
pub use layout::{Hinting, LayoutMetrics, TextBrush, TextLayout};

mod selection;
pub use selection::{len_utf8_from_first_byte, Selectable, StringCursor, TextWithSelection};

// mod movement;

mod edit;
pub use edit::TextEditor;

mod backspace;
pub use backspace::offset_for_delete_backwards;
