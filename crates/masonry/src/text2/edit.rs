// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut, Range};

use kurbo::Point;
use parley::FontContext;
use vello::Scene;
use winit::{
    event::MouseButton,
    keyboard::{Key, NamedKey},
};

use crate::{event::PointerState, Handled, TextEvent};

use super::{
    selection::{Affinity, Selection},
    Selectable, TextWithSelection,
};

/// Text which can be edited
pub trait EditableText: Selectable {
    /// Replace range with new text.
    /// Can panic if supplied an invalid range.
    // TODO: make this generic over Self
    fn edit(&mut self, range: Range<usize>, new: impl Into<String>);
    /// Create a value of this struct
    fn from_str(s: &str) -> Self;
}

impl EditableText for String {
    fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
        self.replace_range(range, &new.into());
    }
    fn from_str(s: &str) -> Self {
        s.to_string()
    }
}

// TODO: What advantage does this actually have?
// impl EditableText for Arc<String> {
//     fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
//         let new = new.into();
//         if !range.is_empty() || !new.is_empty() {
//             Arc::make_mut(self).edit(range, new)
//         }
//     }
//     fn from_str(s: &str) -> Self {
//         Arc::new(s.to_owned())
//     }
// }

/// A region of text which can support editing operations
pub struct TextEditor<T: EditableText> {
    selection: TextWithSelection<T>,
    /// The range of the preedit region in the text
    /// This is currently unused
    _preedit_range: Option<Range<usize>>,
}

impl<T: EditableText> TextEditor<T> {
    pub fn new(text: T, text_size: f32) -> Self {
        Self {
            selection: TextWithSelection::new(text, text_size),
            _preedit_range: None,
        }
    }

    pub fn rebuild(&mut self, fcx: &mut FontContext) {
        // TODO: Add the pre-edit range as an underlined region in the text attributes
        self.selection.rebuild(fcx);
    }

    pub fn draw(&mut self, scene: &mut Scene, point: impl Into<Point>) {
        self.selection.draw(scene, point);
    }

    pub fn pointer_down(
        &mut self,
        origin: Point,
        state: &PointerState,
        button: MouseButton,
    ) -> bool {
        // TODO: If we have a selection and we're hovering over it,
        // implement (optional?) click and drag
        self.selection.pointer_down(origin, state, button)
    }

    pub fn text_event(&mut self, event: &TextEvent) -> Handled {
        let inner_handled = self.selection.text_event(event);
        if inner_handled.is_handled() {
            return inner_handled;
        }
        match event {
            TextEvent::KeyboardKey(event, mods) if event.state.is_pressed() => {
                // We don't input actual text when these keys are pressed
                if !(mods.control_key() || mods.alt_key() || mods.super_key()) {
                    match &event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            eprintln!("Got backspace, not yet handled");
                            Handled::No
                        }
                        Key::Named(NamedKey::Space) => {
                            if let Some(selection) = self.selection.selection {
                                // TODO: We know this is not the fullest model of copy-paste, and that we should work with the inner text
                                // e.g. to put HTML code if supported by the rich text kind
                                let c = ' ';
                                self.text_mut().edit(selection.range(), c);
                                self.selection.selection = Some(Selection::caret(
                                    selection.min() + c.len_utf8(),
                                    // We have just added this character, so we are "affined" with it
                                    Affinity::Downstream,
                                ));
                                Handled::Yes
                            } else {
                                debug_panic!("Got text input event whilst not focused");
                                Handled::No
                            }
                        }
                        Key::Named(_) => Handled::No,
                        Key::Character(c) => {
                            if let Some(selection) = self.selection.selection {
                                // TODO: We know this is not the fullest model of copy-paste, and that we should work with the inner text
                                // e.g. to put HTML code if supported by the rich text kind
                                self.text_mut().edit(selection.range(), &**c);
                                self.selection.selection = Some(Selection::caret(
                                    selection.min() + c.len(),
                                    // We have just added this character, so we are "affined" with it
                                    Affinity::Downstream,
                                ));
                                Handled::Yes
                            } else {
                                debug_panic!("Got text input event whilst not focused");
                                Handled::No
                            }
                        }
                        Key::Unidentified(_) => Handled::No,
                        Key::Dead(d) => {
                            eprintln!("Got dead key {d:?}. Will handle");
                            Handled::No
                        }
                    }
                } else {
                    Handled::No
                }
            }
            TextEvent::KeyboardKey(_, _) => Handled::No,
            TextEvent::Ime(_) => {
                eprintln!(
                    "Got IME event in Textbox. We are planning on supporting this, but do not yet"
                );
                Handled::No
            }
            TextEvent::ModifierChange(_) => Handled::No,
            TextEvent::FocusChange(_) => Handled::No,
        }
    }
}

impl<T: EditableText> Deref for TextEditor<T> {
    type Target = TextWithSelection<T>;

    fn deref(&self) -> &Self::Target {
        &self.selection
    }
}

// TODO: Being able to call `Self::Target::rebuild` (and `draw`) isn't great.
impl<T: EditableText> DerefMut for TextEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.selection
    }
}

#[cfg(test)]
mod tests {
    use super::EditableText;

    // #[test]
    // fn arcstring_empty_edit() {
    //     let a = Arc::new("hello".to_owned());
    //     let mut b = a.clone();
    //     b.edit(5..5, "");
    //     assert!(Arc::ptr_eq(&a, &b));
    // }

    #[test]
    fn replace() {
        let mut a = String::from("hello world");
        a.edit(1..9, "era");
        assert_eq!("herald", a);
    }
}
