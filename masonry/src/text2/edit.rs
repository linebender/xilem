// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut, Range};

use kurbo::Point;
use parley::{FontContext, LayoutContext};
use vello::Scene;
use winit::{
    event::Ime,
    keyboard::{Key, NamedKey},
};

use crate::{
    event::{PointerButton, PointerState},
    Action, EventCtx, Handled, TextEvent,
};

use super::{
    offset_for_delete_backwards,
    selection::{Affinity, Selection},
    Selectable, TextBrush, TextWithSelection,
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
    inner: TextWithSelection<T>,
    /// The range of the preedit region in the text
    preedit_range: Option<Range<usize>>,
}

impl<T: EditableText> TextEditor<T> {
    pub fn new(text: T, text_size: f32) -> Self {
        Self {
            inner: TextWithSelection::new(text, text_size),
            preedit_range: None,
        }
    }

    pub fn reset_preedit(&mut self) {
        self.preedit_range = None;
    }

    /// Rebuild the text.
    ///
    /// See also [TextLayout::rebuild](crate::text2::TextLayout::rebuild) for more comprehensive docs.
    pub fn rebuild(
        &mut self,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<TextBrush>,
    ) {
        self.inner
            .rebuild_with_attributes(font_ctx, layout_ctx, |mut builder| {
                if let Some(range) = self.preedit_range.as_ref() {
                    builder.push(
                        &parley::style::StyleProperty::Underline(true),
                        range.clone(),
                    );
                }
                builder
            });
    }

    pub fn draw(&mut self, scene: &mut Scene, point: impl Into<Point>) {
        self.inner.draw(scene, point);
    }

    pub fn pointer_down(
        &mut self,
        origin: Point,
        state: &PointerState,
        button: PointerButton,
    ) -> bool {
        // TODO: If we have a selection and we're hovering over it,
        // implement (optional?) click and drag
        self.inner.pointer_down(origin, state, button)
    }

    pub fn text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) -> Handled {
        let inner_handled = self.inner.text_event(event);
        if inner_handled.is_handled() {
            return inner_handled;
        }
        match event {
            TextEvent::KeyboardKey(event, mods) if event.state.is_pressed() => {
                // We don't input actual text when these keys are pressed
                if !(mods.control_key() || mods.alt_key() || mods.super_key()) {
                    match &event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            if let Some(selection) = self.inner.selection {
                                if !selection.is_caret() {
                                    self.text_mut().edit(selection.range(), "");
                                    self.inner.selection =
                                        Some(Selection::caret(selection.min(), Affinity::Upstream));
                                } else {
                                    // TODO: more specific behavior may sometimes be warranted here
                                    //       because whole EGCs are more coarse than what people expect
                                    //       to be able to delete individual indic grapheme cluster
                                    //       components among other things.
                                    let text = self.text_mut();
                                    let offset =
                                        offset_for_delete_backwards(selection.active, text);
                                    self.text_mut().edit(offset..selection.active, "");
                                    self.inner.selection =
                                        Some(Selection::caret(offset, selection.active_affinity));
                                }
                                Handled::Yes
                            } else {
                                Handled::No
                            }
                        }
                        Key::Named(NamedKey::Delete) => {
                            if let Some(selection) = self.inner.selection {
                                if !selection.is_caret() {
                                    self.text_mut().edit(selection.range(), "");
                                    self.inner.selection = Some(Selection::caret(
                                        selection.min(),
                                        Affinity::Downstream,
                                    ));
                                } else if let Some(offset) =
                                    self.text().next_grapheme_offset(selection.active)
                                {
                                    self.text_mut().edit(selection.min()..offset, "");
                                    self.inner.selection = Some(Selection::caret(
                                        selection.min(),
                                        selection.active_affinity,
                                    ));
                                }
                                Handled::Yes
                            } else {
                                Handled::No
                            }
                        }
                        Key::Named(NamedKey::Space) => {
                            let selection = self.inner.selection.unwrap_or(Selection {
                                anchor: 0,
                                active: 0,
                                active_affinity: Affinity::Downstream,
                                h_pos: None,
                            });
                            let c = ' ';
                            self.text_mut().edit(selection.range(), c);
                            self.inner.selection = Some(Selection::caret(
                                selection.min() + c.len_utf8(),
                                // We have just added this character, so we are "affined" with it
                                Affinity::Downstream,
                            ));
                            let contents = self.text().as_str().to_string();
                            ctx.submit_action(Action::TextChanged(contents));
                            Handled::Yes
                        }
                        Key::Named(NamedKey::Enter) => {
                            let contents = self.text().as_str().to_string();
                            ctx.submit_action(Action::TextEntered(contents));
                            Handled::Yes
                        }
                        Key::Named(_) => Handled::No,
                        Key::Character(c) => {
                            let selection = self.inner.selection.unwrap_or(Selection {
                                anchor: 0,
                                active: 0,
                                active_affinity: Affinity::Downstream,
                                h_pos: None,
                            });
                            self.text_mut().edit(selection.range(), &**c);
                            self.inner.selection = Some(Selection::caret(
                                selection.min() + c.len(),
                                // We have just added this character, so we are "affined" with it
                                Affinity::Downstream,
                            ));
                            let contents = self.text().as_str().to_string();
                            ctx.submit_action(Action::TextChanged(contents));
                            Handled::Yes
                        }
                        Key::Unidentified(_) => Handled::No,
                        Key::Dead(d) => {
                            eprintln!("Got dead key {d:?}. Will handle");
                            Handled::No
                        }
                    }
                } else if mods.control_key() || mods.super_key()
                // TODO: do things differently on mac, rather than capturing both super and control.
                {
                    match &event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            if let Some(selection) = self.inner.selection {
                                if !selection.is_caret() {
                                    self.text_mut().edit(selection.range(), "");
                                    self.inner.selection =
                                        Some(Selection::caret(selection.min(), Affinity::Upstream));
                                }
                                let offset =
                                    self.text().prev_word_offset(selection.active).unwrap_or(0);
                                self.text_mut().edit(offset..selection.active, "");
                                self.inner.selection =
                                    Some(Selection::caret(offset, Affinity::Upstream));

                                let contents = self.text().as_str().to_string();
                                ctx.submit_action(Action::TextChanged(contents));
                                Handled::Yes
                            } else {
                                Handled::No
                            }
                        }
                        Key::Named(NamedKey::Delete) => {
                            if let Some(selection) = self.inner.selection {
                                if !selection.is_caret() {
                                    self.text_mut().edit(selection.range(), "");
                                    self.inner.selection = Some(Selection::caret(
                                        selection.min(),
                                        Affinity::Downstream,
                                    ));
                                } else if let Some(offset) =
                                    self.text().next_word_offset(selection.active)
                                {
                                    self.text_mut().edit(selection.active..offset, "");
                                    self.inner.selection =
                                        Some(Selection::caret(selection.min(), Affinity::Upstream));
                                }
                                let contents = self.text().as_str().to_string();
                                ctx.submit_action(Action::TextChanged(contents));
                                Handled::Yes
                            } else {
                                Handled::No
                            }
                        }
                        _ => Handled::No,
                    }
                } else {
                    Handled::No
                }
            }
            TextEvent::KeyboardKey(_, _) => Handled::No,
            TextEvent::Ime(ime) => match ime {
                Ime::Commit(text) => {
                    if let Some(selection_range) = self.selection.map(|x| x.range()) {
                        self.text_mut().edit(selection_range.clone(), text);
                        self.selection = Some(Selection::caret(
                            selection_range.start + text.len(),
                            Affinity::Upstream,
                        ));
                    }
                    let contents = self.text().as_str().to_string();
                    ctx.submit_action(Action::TextChanged(contents));
                    Handled::Yes
                }
                Ime::Preedit(preedit_string, preedit_sel) => {
                    if let Some(preedit) = self.preedit_range.clone() {
                        // TODO: Handle the case where this is the same value, to avoid some potential infinite loops
                        self.text_mut().edit(preedit.clone(), preedit_string);
                        let np = preedit.start..(preedit.start + preedit_string.len());
                        self.preedit_range = if preedit_string.is_empty() {
                            None
                        } else {
                            Some(np.clone())
                        };
                        self.selection = if let Some(pec) = preedit_sel {
                            Some(Selection::new(
                                np.start + pec.0,
                                np.start + pec.1,
                                Affinity::Upstream,
                            ))
                        } else {
                            Some(Selection::caret(np.end, Affinity::Upstream))
                        };
                    } else {
                        // If we've been sent an event to clear the preedit,
                        // but there was no existing pre-edit, there's nothing to do
                        // so we report that the event has been handled
                        // An empty preedit is sent by some environments when the
                        // context of a text input has changed, even if the contents
                        // haven't; this also avoids some potential infinite loops
                        if preedit_string.is_empty() {
                            return Handled::Yes;
                        }
                        let sr = self.selection.map(|x| x.range()).unwrap_or(0..0);
                        self.text_mut().edit(sr.clone(), preedit_string);
                        let np = sr.start..(sr.start + preedit_string.len());
                        self.preedit_range = if preedit_string.is_empty() {
                            None
                        } else {
                            Some(np.clone())
                        };
                        self.selection = if let Some(pec) = preedit_sel {
                            Some(Selection::new(
                                np.start + pec.0,
                                np.start + pec.1,
                                Affinity::Upstream,
                            ))
                        } else {
                            Some(Selection::caret(np.start, Affinity::Upstream))
                        };
                    }
                    Handled::Yes
                }
                Ime::Enabled => {
                    // Generally this shouldn't happen, but I can't prove it won't.
                    if let Some(preedit) = self.preedit_range.clone() {
                        self.text_mut().edit(preedit.clone(), "");
                        self.selection = Some(
                            self.selection
                                .unwrap_or(Selection::caret(0, Affinity::Upstream)),
                        );
                        self.preedit_range = None;
                    }
                    Handled::Yes
                }
                Ime::Disabled => {
                    if let Some(preedit) = self.preedit_range.clone() {
                        self.text_mut().edit(preedit.clone(), "");
                        self.preedit_range = None;
                        let sm = self.selection.map(|x| x.min()).unwrap_or(0);
                        if preedit.contains(&sm) {
                            self.selection =
                                Some(Selection::caret(preedit.start, Affinity::Upstream));
                        }
                    }
                    Handled::Yes
                }
            },
            TextEvent::ModifierChange(_) => Handled::No,
            TextEvent::FocusChange(_) => Handled::No,
        }
    }
}

impl<T: EditableText> Deref for TextEditor<T> {
    type Target = TextWithSelection<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// TODO: Being able to call `Self::Target::rebuild` (and `draw`) isn't great.
impl<T: EditableText> DerefMut for TextEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
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
