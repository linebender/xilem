// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget component that integrates with the platform text system.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ops::Range;
use std::sync::{Arc, Weak};

use kurbo::{Line, Point, Vec2};
use parley::layout::Alignment;
use winit::event::Modifiers;

use super::backspace::offset_for_delete_backwards;
use super::editable_text::EditableText;
use super::layout::TextLayout;
use super::shell_text::{Action, Direction, Movement, Selection};
use super::storage::TextStorage;

use crate::text;

/// A widget that accepts text input.
///
/// This is intended to be used as a component of other widgets.
///
/// Text input is more complicated than you think, probably. For a good
/// overview, see [`druid_shell::text`].
///
/// This type manages an inner [`EditSession`] that is shared with the platform.
/// Unlike other aspects of Masonry, the platform interacts with this session, not
/// through discrete events.
///
/// This is managed through a simple 'locking' mechanism; the platform asks for
/// a lock on a particular text session that it wishes to interact with, calls
/// methods on the locked session, and then later releases the lock.
///
/// Importantly, *other events may be received while the lock is held*.
///
/// It is the responsibility of the user of this widget to ensure that the
/// session is not locked before it is accessed. This can be done by checking
/// [`TextComponent::can_read`] and [`TextComponent::can_write`];
/// after checking these methods the inner session can be accessed via
/// [`TextComponent::borrow`] and [`TextComponent::borrow_mut`].
///
/// Semantically, this functions like a `RefCell`; attempting to borrow while
/// a lock is held will result in a panic.
#[derive(Debug, Clone)]
pub struct TextComponent<T> {
    edit_session: Arc<RefCell<EditSession<T>>>,
    lock: Arc<Cell<ImeLock>>,
    // HACK: because of the way focus works (it is managed higher up, in
    // whatever widget is controlling this) we can't rely on `is_focused` in
    // the PaintCtx.
    /// A manual flag set by the parent to control drawing behaviour.
    ///
    /// The parent should update this when handling [`StatusChange::FocusChanged`].
    pub has_focus: bool,
}

// crate::declare_widget!(
//     TextComponentMut,
//     TextComponent<T: (TextStorage + EditableText)>
// );

/// Editable text state.
///
/// This is the inner state of a [`TextComponent`]. It should only be accessed
/// through its containing [`TextComponent`], or by the platform through an
/// [`ImeHandlerRef`] created by [`TextComponent::input_handler`].
#[derive(Debug, Clone)]
pub struct EditSession<T> {
    /// The inner [`TextLayout`] object.
    ///
    /// This is exposed so that users can do things like set text properties;
    /// you should avoid doing things like rebuilding this layout manually, or
    /// setting the text directly.
    pub layout: TextLayout<T>,
    /// If the platform modifies the text, this contains the new text;
    /// we update the app `Data` with this text on the next update pass.
    external_text_change: Option<T>,
    external_selection_change: Option<Selection>,
    external_scroll_to: Option<bool>,
    // external_action: Option<Action>,
    /// A flag set in `update` if the text has changed from a non-IME source.
    // pending_ime_invalidation: Option<ImeInvalidation>,
    /// If `true`, the component will send the [`TextComponent::RETURN`]
    /// notification when the user enters a newline.
    pub send_notification_on_return: bool,
    /// If `true`, the component will send the [`TextComponent::CANCEL`]
    /// notification when the user cancels editing.
    pub send_notification_on_cancel: bool,
    selection: Selection,
    accepts_newlines: bool,
    accepts_tabs: bool,
    alignment: Alignment,
    /// The y-position of the text when it does not fill our width.
    alignment_offset: f64,
    /// The portion of the text that is currently marked by the IME.
    composition_range: Option<Range<usize>>,
    drag_granularity: DragGranularity,
    /// The origin of the textbox, relative to the origin of the window.
    pub origin: Point,
}

/// An object that can be used to acquire an `ImeHandler`.
///
/// This does not own the session; when the widget that owns the session
/// is dropped, this will become invalid.
#[derive(Debug, Clone)]
struct EditSessionRef<T> {
    inner: Weak<RefCell<EditSession<T>>>,
    lock: Arc<Cell<ImeLock>>,
}

/// A locked handle to an [`EditSession`].
///
/// This type implements [`InputHandler`]; it is the type that we pass to the
/// platform.
struct EditSessionHandle<T> {
    text: T,
    inner: Arc<RefCell<EditSession<T>>>,
}

/// When a drag follows a double- or triple-click, the behaviour of
/// drag changes to only select whole words or whole paragraphs.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragGranularity {
    Grapheme,
    /// Start and end are the start/end bounds of the initial selection.
    Word {
        start: usize,
        end: usize,
    },
    /// Start and end are the start/end bounds of the initial selection.
    Paragraph {
        start: usize,
        end: usize,
    },
}

/// An informal lock.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ImeLock {
    None,
    ReadWrite,
    Read,
}

impl<T> TextComponent<T> {
    /// Returns `true` if the inner [`EditSession`] can be read.
    pub fn can_read(&self) -> bool {
        self.lock.get() != ImeLock::ReadWrite
    }

    /// Returns `true` if the inner [`EditSession`] can be mutated.
    pub fn can_write(&self) -> bool {
        self.lock.get() == ImeLock::None
    }

    /// Returns `true` if the IME is actively composing (or the text is locked.)
    ///
    /// When text is composing, you should avoid doing things like modifying the
    /// selection or copy/pasting text.
    pub fn is_composing(&self) -> bool {
        self.can_read() && self.borrow().composition_range.is_some()
    }

    /// Attempt to mutably borrow the inner [`EditSession`].
    ///
    /// # Panics
    ///
    /// This method panics if there is an outstanding lock on the session.
    pub fn borrow_mut(&self) -> RefMut<'_, EditSession<T>> {
        assert!(self.can_write());
        self.edit_session.borrow_mut()
    }

    /// Attempt to borrow the inner [`EditSession`].
    ///
    /// # Panics
    ///
    /// This method panics if there is an outstanding write lock on the session.
    pub fn borrow(&self) -> Ref<'_, EditSession<T>> {
        assert!(self.can_read());
        self.edit_session.borrow()
    }
}

impl<T: TextStorage + EditableText> TextComponentMut<'_, T> {
    pub fn set_text(&mut self, new_text: impl Into<T>) {
        let new_text = new_text.into();
        // TODO - use '==' instead
        let needs_rebuild = self
            .widget
            .borrow()
            .layout
            .text()
            .map(|old| !old.maybe_eq(&new_text))
            .unwrap_or(true);
        if needs_rebuild {
            self.widget.borrow_mut().layout.set_text(new_text.clone());
            self.ctx.request_layout();
        }
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.widget.has_focus = focused;
        self.ctx.request_paint();
    }
}

// impl<T: TextStorage + EditableText> Widget for TextComponent<T> {
//     fn on_event(&mut self, ctx: &mut EventCtx, event: &Event) {
//         match event {
//             Event::MouseDown(mouse) if self.can_write() && !ctx.is_disabled() => {
//                 ctx.set_active(true);
//                 self.borrow_mut()
//                     .do_mouse_down(mouse.pos, mouse.mods, mouse.count);
//                 self.borrow_mut()
//                     .update_pending_invalidation(ImeInvalidation::SelectionChanged);
//                 ctx.request_layout();
//                 ctx.request_paint();
//             }
//             Event::MouseMove(mouse) if self.can_write() => {
//                 if !ctx.is_disabled() {
//                     ctx.set_cursor(&Cursor::IBeam);
//                     if ctx.is_active() {
//                         let pre_sel = self.borrow().selection();
//                         self.borrow_mut().do_drag(mouse.pos);
//                         if self.borrow().selection() != pre_sel {
//                             self.borrow_mut()
//                                 .update_pending_invalidation(ImeInvalidation::SelectionChanged);
//                             ctx.request_layout();
//                             ctx.request_paint();
//                         }
//                     }
//                 } else {
//                     ctx.set_disabled(false);
//                     ctx.clear_cursor();
//                 }
//             }
//             Event::MouseUp(_) if ctx.is_active() => {
//                 ctx.set_active(false);
//                 ctx.request_paint();
//             }
//             Event::ImeStateChange => {
//                 assert!(
//                     self.can_write(),
//                     "lock release should be cause of ImeStateChange event"
//                 );

//                 let scroll_to = self.borrow_mut().take_scroll_to();
//                 if let Some(scroll_to) = scroll_to {
//                     ctx.submit_notification(TextComponent::SCROLL_TO.with(scroll_to));
//                 }

//                 let text = self.borrow_mut().take_external_text_change();
//                 if let Some(text) = text {
//                     self.borrow_mut().layout.set_text(text.clone());
//                     let new_text = self
//                         .borrow()
//                         .layout
//                         .text()
//                         .map(|txt| txt.as_str())
//                         .unwrap_or("")
//                         .to_string();
//                     ctx.submit_notification(TextComponent::TEXT_CHANGED.with(new_text));
//                 }

//                 let action = self.borrow_mut().take_external_action();
//                 if let Some(action) = action {
//                     match action {
//                         Action::Cancel => ctx.submit_notification(TextComponent::CANCEL),
//                         Action::InsertNewLine { .. } => {
//                             let text = self
//                                 .borrow()
//                                 .layout
//                                 .text()
//                                 .map(|txt| txt.as_str())
//                                 .unwrap_or("")
//                                 .to_string();
//                             ctx.submit_notification(TextComponent::RETURN.with(text));
//                         }
//                         Action::InsertTab { .. } => ctx.submit_notification(TextComponent::TAB),
//                         Action::InsertBacktab => {
//                             ctx.submit_notification(TextComponent::BACKTAB)
//                         }
//                         _ => tracing::warn!("unexpected external action '{:?}'", action),
//                     };
//                 }

//                 let selection = self.borrow_mut().take_external_selection_change();
//                 if let Some(selection) = selection {
//                     self.borrow_mut().selection = selection;
//                     ctx.request_paint();
//                 }
//                 ctx.request_layout();
//             }
//             _ => (),
//         }
//     }

//     fn on_status_change(&mut self, _ctx: &mut LifeCycleCtx, _event: &StatusChange) {}

//     fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
//         match event {
//             LifeCycle::WidgetAdded => {
//                 assert!(
//                     self.can_write(),
//                     "ime should never be locked at WidgetAdded"
//                 );
//                 self.borrow_mut().layout.rebuild_if_needed(ctx.text());
//             }
//             LifeCycle::DisabledChanged(disabled) => {
//                 if self.can_write() {
//                     let color = if *disabled {
//                         theme::DISABLED_TEXT_COLOR
//                     } else {
//                         theme::TEXT_COLOR
//                     };
//                     self.borrow_mut().layout.set_text_color(color);
//                 }
//                 ctx.request_layout();
//             }
//             //FIXME: this should happen in the parent too?
//             LifeCycle::Internal(crate::InternalLifeCycle::ParentWindowOrigin) => {
//                 if self.can_write() {
//                     let prev_origin = self.borrow().origin;
//                     let new_origin = ctx.window_origin();
//                     if prev_origin != new_origin {
//                         self.borrow_mut().origin = ctx.window_origin();
//                         ctx.invalidate_text_input(ImeInvalidation::LayoutChanged);
//                     }
//                 }
//             }
//             _ => (),
//         }
//     }

//     fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
//         if !self.can_write() {
//             tracing::warn!("Text layout called with IME lock held.");
//             return Size::ZERO;
//         }

//         self.borrow_mut().layout.set_wrap_width(bc.max().width);
//         self.borrow_mut().layout.rebuild_if_needed(ctx.text());
//         let metrics = self.borrow().layout.layout_metrics();
//         let width = if bc.max().width.is_infinite() || bc.max().width < f64::MAX {
//             metrics.trailing_whitespace_width
//         } else {
//             metrics.size.width
//         };
//         let size = bc.constrain((width, metrics.size.height));
//         let extra_width = if self.borrow().accepts_newlines {
//             0.0
//         } else {
//             (size.width - width).max(0.0)
//         };
//         self.borrow_mut().update_alignment_offset(extra_width);
//         let baseline_off = metrics.size.height - metrics.first_baseline;
//         ctx.set_baseline_offset(baseline_off);
//         size
//     }

//     fn paint(&mut self, ctx: &mut PaintCtx) {
//         if !self.can_read() {
//             tracing::warn!("Text paint called with IME lock held.");
//         }

//         let selection_color = if self.has_focus {
//             theme::SELECTED_TEXT_BACKGROUND_COLOR
//         } else {
//             theme::SELECTED_TEXT_INACTIVE_BACKGROUND_COLOR
//         };

//         let cursor_color = theme::CURSOR_COLOR;
//         let text_offset = Vec2::new(self.borrow().alignment_offset, 0.0);

//         let selection = self.borrow().selection();
//         let composition = self.borrow().composition_range();
//         let sel_rects = self.borrow().layout.rects_for_range(selection.range());
//         if let Some(composition) = composition {
//             // I believe selection should always be contained in composition range while composing?
//             assert!(composition.start <= selection.anchor && composition.end >= selection.active);
//             let comp_rects = self.borrow().layout.rects_for_range(composition);
//             for region in comp_rects {
//                 let y = region.max_y().floor();
//                 let line = Line::new((region.min_x(), y), (region.max_x(), y)) + text_offset;
//                 ctx.stroke(line, &cursor_color, 1.0);
//             }
//             for region in sel_rects {
//                 let y = region.max_y().floor();
//                 let line = Line::new((region.min_x(), y), (region.max_x(), y)) + text_offset;
//                 ctx.stroke(line, &cursor_color, 2.0);
//             }
//         } else {
//             for region in sel_rects {
//                 let rounded = (region + text_offset).to_rounded_rect(1.0);
//                 ctx.fill(rounded, &selection_color);
//             }
//         }
//         self.borrow().layout.draw(ctx, text_offset.to_point());
//     }

//     fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
//         SmallVec::new()
//     }

//     fn make_trace_span(&self) -> Span {
//         trace_span!("TextComponent")
//     }
// }

impl<T> EditSession<T> {
    /// The current [`Selection`].
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Manually set the selection.
    ///
    /// If the new selection is different from the current selection, this
    /// will return an ime event that the controlling widget should use to
    /// invalidate the platform's IME state, by passing it to
    /// [`EventCtx::invalidate_text_input`].
    #[must_use]
    pub fn set_selection(&mut self, selection: Selection) /* -> Option<ImeInvalidation> */
    {
        if selection != self.selection {
            self.selection = selection;
            // self.update_pending_invalidation(ImeInvalidation::SelectionChanged);
            // Some(ImeInvalidation::SelectionChanged)
        }
    }

    /// The range of text currently being modified by an IME.
    pub fn composition_range(&self) -> Option<Range<usize>> {
        self.composition_range.clone()
    }

    /// Sets whether or not this session will allow the insertion of newlines.
    pub fn set_accepts_newlines(&mut self, accepts_newlines: bool) {
        self.accepts_newlines = accepts_newlines;
    }

    /// Set the text alignment.
    ///
    /// This is only meaningful for single-line text that does not fill
    /// the minimum layout size.
    pub fn set_text_alignment(&mut self, alignment: Alignment) {
        self.alignment = alignment;
    }

    /// The text alignment.
    pub fn text_alignment(&self) -> Alignment {
        self.alignment
    }

    // /// Returns any invalidation action that should be passed to the platform.
    // ///
    // /// The user of this component *must* check this after calling `update`.
    // pub fn pending_ime_invalidation(&mut self) -> Option<ImeInvalidation> {
    //     self.pending_ime_invalidation.take()
    // }

    fn take_external_text_change(&mut self) -> Option<T> {
        self.external_text_change.take()
    }

    fn take_external_selection_change(&mut self) -> Option<Selection> {
        self.external_selection_change.take()
    }

    fn take_scroll_to(&mut self) -> Option<bool> {
        self.external_scroll_to.take()
    }

    // fn take_external_action(&mut self) -> Option<Action> {
    //     self.external_action.take()
    // }

    // // we don't want to replace a more aggressive invalidation with a less aggressive one.
    // fn update_pending_invalidation(&mut self, new_invalidation: ImeInvalidation) {
    //     self.pending_ime_invalidation = match self.pending_ime_invalidation.take() {
    //         None => Some(new_invalidation),
    //         Some(prev) => match (prev, new_invalidation) {
    //             (ImeInvalidation::SelectionChanged, ImeInvalidation::SelectionChanged) => {
    //                 ImeInvalidation::SelectionChanged
    //             }
    //             (ImeInvalidation::LayoutChanged, ImeInvalidation::LayoutChanged) => {
    //                 ImeInvalidation::LayoutChanged
    //             }
    //             _ => ImeInvalidation::Reset,
    //         }
    //         .into(),
    //     }
    // }

    fn update_alignment_offset(&mut self, extra_width: f64) {
        self.alignment_offset = match self.alignment {
            Alignment::Start | Alignment::Justified => 0.0,
            Alignment::End => extra_width,
            Alignment::Center => extra_width / 2.0,
        };
    }
}

impl<T: TextStorage + EditableText> EditSession<T> {
    /// Insert text *not* from the IME, replacing the current selection.
    ///
    /// The caller is responsible for notifying the platform of the change in
    /// text state, by calling [`EventCtx::invalidate_text_input`].
    #[must_use]
    pub fn insert_text(&mut self, data: &mut T, new_text: &str) {
        let new_cursor_pos = self.selection.min() + new_text.len();
        data.edit(self.selection.range(), new_text);
        self.selection = Selection::caret(new_cursor_pos);
        self.scroll_to_selection_end(true);
    }

    /// Sets the clipboard to the contents of the current selection.
    ///
    /// Returns `true` if the clipboard was set, and `false` if not (indicating)
    /// that the selection was empty.)
    pub fn set_clipboard(&self) -> bool {
        // if let Some(text) = self
        //     .layout
        //     .text()
        //     .and_then(|txt| txt.slice(self.selection.range()))
        // {
        //     if !text.is_empty() {
        //         druid_shell::Application::global()
        //             .clipboard()
        //             .put_string(text);
        //         return true;
        //     }
        // }
        false
    }

    fn scroll_to_selection_end(&mut self, after_edit: bool) {
        self.external_scroll_to = Some(after_edit);
    }

    fn do_action(&mut self, buffer: &mut T, action: Action) {
        match action {
            Action::Move(movement) => {
                let sel = text::movement::movement(movement, self.selection, &self.layout, false);
                self.external_selection_change = Some(sel);
                self.scroll_to_selection_end(false);
            }
            Action::MoveSelecting(movement) => {
                let sel = text::movement::movement(movement, self.selection, &self.layout, true);
                self.external_selection_change = Some(sel);
                self.scroll_to_selection_end(false);
            }
            Action::SelectAll => {
                let len = buffer.len();
                self.external_selection_change = Some(Selection::new(0, len));
            }
            Action::SelectWord => {
                if self.selection.is_caret() {
                    let range =
                        text::movement::word_range_for_pos(buffer.as_str(), self.selection.active);
                    self.external_selection_change = Some(Selection::new(range.start, range.end));
                }

                // it is unclear what the behaviour should be if the selection
                // is not a caret (and may span multiple words)
            }
            // This requires us to have access to the layout, which might be stale?
            Action::SelectLine => (),
            // this assumes our internal selection is consistent with the buffer?
            Action::SelectParagraph => {
                if !self.selection.is_caret() || buffer.len() < self.selection.active {
                    return;
                }
                let prev = buffer.preceding_line_break(self.selection.active);
                let next = buffer.next_line_break(self.selection.active);
                self.external_selection_change = Some(Selection::new(prev, next));
            }
            Action::Delete(movement) if self.selection.is_caret() => {
                if movement == Movement::Grapheme(Direction::Upstream) {
                    self.backspace(buffer);
                } else {
                    let to_delete =
                        text::movement::movement(movement, self.selection, &self.layout, true);
                    self.selection = to_delete;
                    self.ime_insert_text(buffer, "")
                }
            }
            Action::Delete(_) => self.ime_insert_text(buffer, ""),
            Action::DecomposingBackspace => {
                tracing::warn!("Decomposing Backspace is not implemented");
                self.backspace(buffer);
            }
            //Action::UppercaseSelection
            //| Action::LowercaseSelection
            //| Action::TitlecaseSelection => {
            //tracing::warn!("IME transformations are not implemented");
            //}
            Action::InsertNewLine {
                newline_type,
                ignore_hotkey,
            } => {
                if self.send_notification_on_return && !ignore_hotkey {
                    self.external_action = Some(action);
                } else if self.accepts_newlines {
                    self.ime_insert_text(buffer, &newline_type.to_string());
                }
            }
            Action::InsertTab { ignore_hotkey } => {
                if ignore_hotkey || self.accepts_tabs {
                    self.ime_insert_text(buffer, "\t");
                } else if !ignore_hotkey {
                    self.external_action = Some(action);
                }
            }
            Action::InsertBacktab => {
                if !self.accepts_tabs {
                    self.external_action = Some(action);
                }
            }
            Action::InsertSingleQuoteIgnoringSmartQuotes => self.ime_insert_text(buffer, "'"),
            Action::InsertDoubleQuoteIgnoringSmartQuotes => self.ime_insert_text(buffer, "\""),
            Action::Cancel if self.send_notification_on_cancel => {
                self.external_action = Some(action)
            }
            other => tracing::warn!("unhandled IME action {:?}", other),
        }
    }

    /// Replace the current selection with `text`, and advance the cursor.
    ///
    /// This should only be called from the IME.
    fn ime_insert_text(&mut self, buffer: &mut T, text: &str) {
        let new_cursor_pos = self.selection.min() + text.len();
        buffer.edit(self.selection.range(), text);
        self.external_selection_change = Some(Selection::caret(new_cursor_pos));
        self.scroll_to_selection_end(true);
    }

    fn backspace(&mut self, buffer: &mut T) {
        let to_del = if self.selection.is_caret() {
            let del_start = offset_for_delete_backwards(&self.selection, buffer);
            del_start..self.selection.anchor
        } else {
            self.selection.range()
        };
        self.external_selection_change = Some(Selection::caret(to_del.start));
        buffer.edit(to_del, "");
        self.scroll_to_selection_end(true);
    }

    fn do_mouse_down(&mut self, point: Point, mods: Modifiers, count: u8) {
        let point = point - Vec2::new(self.alignment_offset, 0.0);
        let pos = self.layout.text_position_for_point(point);
        if mods.shift() {
            self.selection.active = pos;
        } else {
            let Range { start, end } = self.sel_region_for_pos(pos, count);
            self.selection = Selection::new(start, end);
            self.drag_granularity = match count {
                2 => DragGranularity::Word { start, end },
                3 => DragGranularity::Paragraph { start, end },
                _ => DragGranularity::Grapheme,
            };
        }
    }

    fn do_drag(&mut self, point: Point) {
        let point = point - Vec2::new(self.alignment_offset, 0.0);
        //FIXME: this should behave differently if we were double or triple clicked
        let pos = self.layout.text_position_for_point(point);
        let text = match self.layout.text() {
            Some(text) => text,
            None => return,
        };

        let (start, end) = match self.drag_granularity {
            DragGranularity::Grapheme => (self.selection.anchor, pos),
            DragGranularity::Word { start, end } => {
                let word_range = self.word_for_pos(pos);
                if pos <= start {
                    (end, word_range.start)
                } else {
                    (start, word_range.end)
                }
            }
            DragGranularity::Paragraph { start, end } => {
                let par_start = text.preceding_line_break(pos);
                let par_end = text.next_line_break(pos);

                if pos <= start {
                    (end, par_start)
                } else {
                    (start, par_end)
                }
            }
        };
        self.selection = Selection::new(start, end);
        self.scroll_to_selection_end(false);
    }

    /// Returns a line suitable for drawing a standard cursor.
    pub fn cursor_line_for_text_position(&self, pos: usize) -> Line {
        let line = self.layout.cursor_line_for_text_position(pos);
        line + Vec2::new(self.alignment_offset, 0.0)
    }

    fn sel_region_for_pos(&mut self, pos: usize, click_count: u8) -> Range<usize> {
        match click_count {
            1 => pos..pos,
            2 => self.word_for_pos(pos),
            _ => {
                let text = match self.layout.text() {
                    Some(text) => text,
                    None => return pos..pos,
                };
                let line_min = text.preceding_line_break(pos);
                let line_max = text.next_line_break(pos);
                line_min..line_max
            }
        }
    }

    fn word_for_pos(&self, pos: usize) -> Range<usize> {
        let layout = match self.layout.layout() {
            Some(layout) => layout,
            None => return pos..pos,
        };

        let line_n = layout.hit_test_text_position(pos).line;
        let lm = layout.line_metric(line_n).unwrap();
        let text = layout.line_text(line_n).unwrap();
        let rel_pos = pos - lm.start_offset;
        let mut range = text::movement::word_range_for_pos(text, rel_pos);
        range.start += lm.start_offset;
        range.end += lm.start_offset;
        range
    }
}

impl<T: TextStorage> EditSessionHandle<T> {
    fn new(inner: Arc<RefCell<EditSession<T>>>) -> Self {
        let text = inner.borrow().layout.text().cloned().unwrap();
        EditSessionHandle { text, inner }
    }
}
