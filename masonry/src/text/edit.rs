// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut, Range};

use parley::{FontContext, LayoutContext};
use tracing::warn;
use vello::kurbo::Point;
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

/// A region of text which can support editing operations
pub struct TextEditor {
    inner: TextWithSelection<String>,
    /// The range of the preedit region in the text
    preedit_range: Option<Range<usize>>,
}

impl TextEditor {
    pub fn new(text: String, text_size: f32) -> Self {
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
    /// See also [`TextLayout::rebuild`](crate::text::TextLayout::rebuild) for more comprehensive docs.
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
                                    self.text_mut().replace_range(selection.range(), "");
                                    self.inner.selection =
                                        Some(Selection::caret(selection.min(), Affinity::Upstream));

                                    let contents = self.text().clone();
                                    ctx.submit_action(Action::TextChanged(contents));
                                } else {
                                    // TODO: more specific behavior may sometimes be warranted here
                                    //       because whole EGCs are more coarse than what people expect
                                    //       to be able to delete individual indic grapheme cluster
                                    //       components among other things.
                                    let text = self.text_mut();
                                    let offset =
                                        offset_for_delete_backwards(selection.active, text);
                                    self.text_mut().replace_range(offset..selection.active, "");
                                    self.inner.selection =
                                        Some(Selection::caret(offset, selection.active_affinity));

                                    let contents = self.text().clone();
                                    ctx.submit_action(Action::TextChanged(contents));
                                }
                                Handled::Yes
                            } else {
                                Handled::No
                            }
                        }
                        Key::Named(NamedKey::Delete) => {
                            if let Some(selection) = self.inner.selection {
                                if !selection.is_caret() {
                                    self.text_mut().replace_range(selection.range(), "");
                                    self.inner.selection = Some(Selection::caret(
                                        selection.min(),
                                        Affinity::Downstream,
                                    ));

                                    let contents = self.text().clone();
                                    ctx.submit_action(Action::TextChanged(contents));
                                } else if let Some(offset) =
                                    self.text().next_grapheme_offset(selection.active)
                                {
                                    self.text_mut().replace_range(selection.min()..offset, "");
                                    self.inner.selection = Some(Selection::caret(
                                        selection.min(),
                                        selection.active_affinity,
                                    ));

                                    let contents = self.text().clone();
                                    ctx.submit_action(Action::TextChanged(contents));
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
                            self.text_mut()
                                .replace_range(selection.range(), &c.to_string());
                            self.inner.selection = Some(Selection::caret(
                                selection.min() + c.len_utf8(),
                                // We have just added this character, so we are "affined" with it
                                Affinity::Downstream,
                            ));
                            let contents = self.text().clone();
                            ctx.submit_action(Action::TextChanged(contents));
                            Handled::Yes
                        }
                        Key::Named(NamedKey::Enter) => {
                            let contents = self.text().clone();
                            ctx.submit_action(Action::TextEntered(contents));
                            Handled::Yes
                        }
                        Key::Named(_) => Handled::No,
                        Key::Character(c) => {
                            self.insert_text(event.text.as_ref().unwrap_or(c), ctx)
                        }
                        Key::Unidentified(_) => match event.text.as_ref() {
                            Some(text) => self.insert_text(text, ctx),
                            None => Handled::No,
                        },
                        Key::Dead(d) => {
                            warn!("Got dead key {d:?}. Will handle");
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
                                    self.text_mut().replace_range(selection.range(), "");
                                    self.inner.selection =
                                        Some(Selection::caret(selection.min(), Affinity::Upstream));
                                }
                                let offset =
                                    self.text().prev_word_offset(selection.active).unwrap_or(0);
                                self.text_mut().replace_range(offset..selection.active, "");
                                self.inner.selection =
                                    Some(Selection::caret(offset, Affinity::Upstream));

                                let contents = self.text().clone();
                                ctx.submit_action(Action::TextChanged(contents));
                                Handled::Yes
                            } else {
                                Handled::No
                            }
                        }
                        Key::Named(NamedKey::Delete) => {
                            if let Some(selection) = self.inner.selection {
                                if !selection.is_caret() {
                                    self.text_mut().replace_range(selection.range(), "");
                                    self.inner.selection = Some(Selection::caret(
                                        selection.min(),
                                        Affinity::Downstream,
                                    ));
                                } else if let Some(offset) =
                                    self.text().next_word_offset(selection.active)
                                {
                                    self.text_mut().replace_range(selection.active..offset, "");
                                    self.inner.selection =
                                        Some(Selection::caret(selection.min(), Affinity::Upstream));
                                }
                                let contents = self.text().clone();
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
                        self.text_mut().replace_range(selection_range.clone(), text);
                        self.selection = Some(Selection::caret(
                            selection_range.start + text.len(),
                            Affinity::Upstream,
                        ));
                    }
                    let contents = self.text().clone();
                    ctx.submit_action(Action::TextChanged(contents));
                    Handled::Yes
                }
                Ime::Preedit(preedit_string, preedit_sel) => {
                    if let Some(preedit) = self.preedit_range.clone() {
                        // TODO: Handle the case where this is the same value, to avoid some potential infinite loops
                        self.text_mut()
                            .replace_range(preedit.clone(), preedit_string);
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
                        self.text_mut().replace_range(sr.clone(), preedit_string);
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
                        self.text_mut().replace_range(preedit.clone(), "");
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
                        self.text_mut().replace_range(preedit.clone(), "");
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

    fn insert_text(&mut self, c: &winit::keyboard::SmolStr, ctx: &mut EventCtx) -> Handled {
        let selection = self.inner.selection.unwrap_or(Selection {
            anchor: 0,
            active: 0,
            active_affinity: Affinity::Downstream,
            h_pos: None,
        });
        self.text_mut().replace_range(selection.range(), c);
        self.inner.selection = Some(Selection::caret(
            selection.min() + c.len(),
            // We have just added this character, so we are "affined" with it
            Affinity::Downstream,
        ));
        let contents = self.text().clone();
        ctx.submit_action(Action::TextChanged(contents));
        Handled::Yes
    }
}

impl Deref for TextEditor {
    type Target = TextWithSelection<String>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// TODO: Being able to call `Self::Target::rebuild` (and `draw`) isn't great.
impl DerefMut for TextEditor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn replace() {
        let mut a = String::from("hello world");
        a.replace_range(1..9, "era");
        assert_eq!("herald", a);
    }
}
