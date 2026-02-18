// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::mem::Discriminant;

use accesskit::{Node, Role};
use parley::PlainEditor;
use parley::editing::{Generation, SplitString};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::keyboard::{Key, KeyState, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, BrushIndex, ChildrenIds, CursorIcon, EventCtx, Ime, LayoutCtx,
    MeasureCtx, PaintCtx, PointerButton, PointerButtonEvent, PointerEvent, PointerUpdate,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, StyleProperty, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, render_text,
};
use crate::kurbo::{Affine, Axis, Point, Rect, Size};
use crate::layout::LenReq;
use crate::peniko::Fill;
use crate::properties::{
    CaretColor, ContentColor, DisabledContentColor, SelectionColor, UnfocusedSelectionColor,
};
use crate::theme::default_text_styles;
use crate::util::bounding_box_to_rect;
use crate::util::debug_panic;
use crate::{TextAlign, theme};

/// `TextArea` implements the core of interactive text.
///
/// It is used to implement [`TextInput`](super::TextInput) and [`Prose`](super::Prose).
/// It is rare that you will use a raw `TextArea` as a widget in your app; most users
/// should prefer one of those wrappers.
///
/// This ensures that the editable and read-only text have the same text selection and
/// copy/paste behaviour.
///
/// The `USER_EDITABLE` const generic parameter determines whether the text area's contents can be
/// edited by the user of the app.
/// This is true for `TextInput` and false for `Prose`.
///
/// This widget emits [`TextAction`] only when `USER_EDITABLE` is true.
///
/// The exact semantics of how much horizontal space this widget takes up has not been determined.
/// In particular, this has consequences when the text alignment is set.
// TODO: RichTextInput 👀
// TODO: Support for links - https://github.com/linebender/xilem/issues/360
pub struct TextArea<const USER_EDITABLE: bool> {
    // TODO: Placeholder text?
    /// The underlying `PlainEditor`, which provides a high-level interface for us to dispatch into.
    editor: PlainEditor<BrushIndex>,
    /// The generation of `editor` which we have rendered.
    ///
    /// TODO: Split into rendered and layout generation. This will make the `edited` mechanism in [`on_text_event`](Widget::on_text_event).
    rendered_generation: Generation,

    /// Whether to wrap words in this area.
    ///
    /// Note that if clipping is desired, that should be added by the parent widget.
    /// Can be set using [`set_word_wrap`](Self::set_word_wrap).
    word_wrap: bool,
    /// The value of `max_advance` when this layout was last calculated.
    ///
    /// If it has changed, we need to re-perform line-breaking.
    last_max_advance: Option<f32>,

    /// Whether to hint whilst drawing the text.
    ///
    /// Should be disabled whilst an animation involving this text is ongoing.
    /// Can be set using [`set_hint`](Self::set_hint).
    // TODO: What classes of animations? I.e does scrolling count?
    hint: bool,

    /// What key combination should trigger a newline insertion.
    /// If this is set to `InsertNewline::OnEnter` then `Enter` will insert a newline and _not_ trigger a [`TextAction::Entered`] event.
    insert_newline: InsertNewline,

    /// Whether to show the cursor, used for the blink animation.
    anim_cursor_visible: bool,

    /// Previous interval (ms), used for the cursor's blink animation.
    anim_prev_interval: u64,

    /// Time elapsed (ms) to calculate the timeout of the cursor's blink animation.
    anim_elapsed: u64,
}

// --- MARK: BUILDERS
impl TextArea<true> {
    /// Creates a new `TextArea` which can be edited.
    ///
    /// Useful for creating a styled [`TextInput`](super::TextInput).
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new_editable(text: &str) -> Self {
        Self::new(text)
    }
}

impl TextArea<false> {
    /// Creates a new `TextArea` which cannot be edited by the user.
    ///
    /// Useful for creating a styled [`Prose`](super::Prose).
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new_immutable(text: &str) -> Self {
        Self::new(text)
    }
}

impl<const EDITABLE: bool> TextArea<EDITABLE> {
    /// Creates a new `TextArea` with the given text and default settings.
    ///
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new(text: &str) -> Self {
        let mut editor = PlainEditor::new(theme::TEXT_SIZE_NORMAL);
        default_text_styles(editor.edit_styles());
        editor.set_text(text);
        Self {
            editor,
            rendered_generation: Generation::default(),
            word_wrap: true,
            last_max_advance: None,
            hint: true,
            insert_newline: InsertNewline::default(),
            anim_cursor_visible: true,
            anim_prev_interval: 0,
            anim_elapsed: 0,
        }
    }

    /// Sets a style property for the new text area.
    ///
    /// Style properties set by this method include [text size](parley::StyleProperty::FontSize),
    /// [font family](parley::StyleProperty::FontStack), [font weight](parley::StyleProperty::FontWeight),
    /// and [variable font parameters](parley::StyleProperty::FontVariations).
    /// The styles inserted here apply to the entire text; we currently do not
    /// support inline rich text.
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`ContentColor`] and [`DisabledContentColor`] properties instead.
    /// This is also not additive for [font stacks](parley::StyleProperty::FontStack), and
    /// instead overwrites any previous font stack.
    ///
    /// To set a style property on an active text area, use [`insert_style`](Self::insert_style).
    #[track_caller]
    pub fn with_style(mut self, property: impl Into<StyleProperty>) -> Self {
        self.insert_style_inner(property.into());
        self
    }

    /// Sets a style property for the new text area, returning the old value.
    ///
    /// Most users should prefer [`with_style`](Self::with_style) instead.
    pub fn try_with_style(
        mut self,
        property: impl Into<StyleProperty>,
    ) -> (Self, Option<StyleProperty>) {
        let old = self.insert_style_inner(property.into());
        (self, old)
    }

    /// Control [word wrapping](https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap) for the new text area.
    ///
    /// When enabled, the text will be laid out to fit within the available width.
    /// If word wrapping is disabled, the text will likely flow past the available area.
    /// Note that parent widgets will often clip this, so the overflow will not be visible.
    ///
    /// This widget does not currently support scrolling to the cursor,
    /// so it is recommended to leave word wrapping enabled.
    ///
    /// To modify this on an active text area, use [`set_word_wrap`](Self::set_word_wrap).
    pub fn with_word_wrap(mut self, wrap_words: bool) -> Self {
        self.word_wrap = wrap_words;
        self
    }

    /// Sets the [text alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    ///
    /// To modify this on an active text area, use [`set_text_alignment`](Self::set_text_alignment).
    pub fn with_text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.editor.set_alignment(text_alignment);
        self
    }

    /// Sets whether [hinting](https://en.wikipedia.org/wiki/Font_hinting) will be used for this text area.
    ///
    /// Hinting is a process where text is drawn "snapped" to pixel boundaries to improve fidelity.
    /// The default is true, i.e. hinting is enabled by default.
    ///
    /// This should be set to false if the text area will be animated at creation.
    /// The kinds of relevant animations include changing variable font parameters,
    /// translating or scaling.
    /// Failing to do so will likely lead to an unpleasant shimmering effect, as different parts of the
    /// text "snap" at different times.
    ///
    /// To modify this on an active text area, use [`set_hint`](Self::set_hint).
    /// You should do so as an animation starts and ends.
    // TODO: Should we tell each widget if smooth scrolling is ongoing so they can disable their hinting?
    // Alternatively, we should automate disabling hinting at the Vello layer when composing.
    pub fn with_hint(mut self, hint: bool) -> Self {
        self.hint = hint;
        self
    }

    /// Configures how this text area handles the user pressing Enter <kbd>↵</kbd>.
    pub fn with_insert_newline(mut self, insert_newline: InsertNewline) -> Self {
        self.insert_newline = insert_newline;
        self
    }

    /// Shared logic between `with_style` and `insert_style`
    #[track_caller]
    fn insert_style_inner(&mut self, property: StyleProperty) -> Option<StyleProperty> {
        if let StyleProperty::Brush(idx @ BrushIndex(1..))
        | StyleProperty::UnderlineBrush(Some(idx @ BrushIndex(1..)))
        | StyleProperty::StrikethroughBrush(Some(idx @ BrushIndex(1..))) = &property
        {
            debug_panic!(
                "Can't set a non-zero brush index ({idx:?}) on a `TextArea`, as it only supports global styling.\n\
                To modify the active brush, use `set_brush` or `with_brush` instead"
            );
            None
        } else {
            self.editor.edit_styles().insert(property)
        }
    }
}

// --- MARK: METHODS
impl<const EDITABLE: bool> TextArea<EDITABLE> {
    /// Returns the current text of this text area.
    ///
    /// To update the text of an active text area, use [`reset_text`](Self::reset_text).
    ///
    /// The return value is not just `&str` to handle IME preedits.
    pub fn text(&self) -> SplitString<'_> {
        self.editor.text()
    }

    /// Check if this text area holds nothing, including IME preedit content.
    pub fn is_empty(&self) -> bool {
        self.editor.raw_text().is_empty()
    }

    /// Returns the IME area from the editor, accounting for padding.
    ///
    /// This should only be called when the editor layout is available.
    fn ime_area(&self) -> Rect {
        debug_assert!(
            self.editor.try_layout().is_some(),
            "TextArea::ime_area should only be called when the editor layout is available"
        );
        bounding_box_to_rect(self.editor.ime_cursor_area())
    }
}

// --- MARK: WIDGETMUT
impl<const EDITABLE: bool> TextArea<EDITABLE> {
    /// Sets font styling for an active text area.
    ///
    /// Style properties set by this method include [text size](parley::StyleProperty::FontSize),
    /// [font family](parley::StyleProperty::FontStack), [font weight](parley::StyleProperty::FontWeight),
    /// and [variable font parameters](parley::StyleProperty::FontVariations).
    /// The styles inserted here apply to the entire text; we currently do not
    /// support inline rich text.
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`ContentColor`] and [`DisabledContentColor`] properties instead.
    /// This is also not additive for [font stacks](parley::StyleProperty::FontStack), and
    /// instead overwrites any previous font stack.
    ///
    /// This is the runtime equivalent of [`with_style`](Self::with_style).
    #[track_caller]
    pub fn insert_style(
        this: &mut WidgetMut<'_, Self>,
        property: impl Into<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.insert_style_inner(property.into());

        this.ctx.request_layout();
        old
    }

    /// [Retains](std::vec::Vec::retain) only the styles for which `f` returns true.
    ///
    /// Styles which are removed return to Parley's default values.
    /// In most cases, these are the defaults for this widget.
    ///
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](parley::StyleProperty::FontSize).
    pub fn retain_styles(this: &mut WidgetMut<'_, Self>, f: impl FnMut(&StyleProperty) -> bool) {
        this.widget.editor.edit_styles().retain(f);

        this.ctx.request_layout();
    }

    /// Removes the style with the discriminant `property`.
    ///
    /// Styles which are removed return to Parley's default values.
    /// In most cases, these are the defaults for this widget.
    ///
    /// To get the discriminant requires constructing a valid `StyleProperty` for the
    /// the desired property and passing it to [`core::mem::discriminant`].
    /// Getting this discriminant is usually possible in a `const` context.
    ///
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](parley::StyleProperty::FontSize).
    pub fn remove_style(
        this: &mut WidgetMut<'_, Self>,
        property: Discriminant<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.editor.edit_styles().remove(property);

        this.ctx.request_layout();
        old
    }

    /// Sets the text displayed in this widget.
    ///
    /// This is likely to be disruptive if the user is focused on this widget,
    /// as it does not retain selections, and may cause undesirable interactions with IME.
    pub fn reset_text(this: &mut WidgetMut<'_, Self>, new_text: &str) {
        // If the IME is currently composing, we need to clear the compose first. This is quite
        // disruptive, but we've warned about that. The platform's state is not reset, and the
        // preedit will show up again when the platform updates it.
        if this.widget.editor.is_composing() {
            let (fctx, lctx) = this.ctx.text_contexts();
            this.widget.editor.driver(fctx, lctx).clear_compose();
        }
        this.widget.editor.set_text(new_text);

        let (fctx, lctx) = this.ctx.text_contexts();
        this.widget.editor.driver(fctx, lctx).move_to_text_end();

        this.ctx.request_layout();
    }

    /// Sets [word wrapping](https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap) for the text area.
    ///
    /// When enabled, the text will be laid out to fit within the available width.
    /// If word wrapping is disabled, the text will likely flow past the available area.
    /// Note that parent widgets will often clip this, so the overflow will not be visible.
    ///
    /// This widget does not currently support scrolling to the cursor,
    /// so it is recommended to leave word wrapping enabled.
    ///
    /// The runtime equivalent of [`with_word_wrap`](Self::with_word_wrap).
    pub fn set_word_wrap(this: &mut WidgetMut<'_, Self>, wrap_words: bool) {
        this.widget.word_wrap = wrap_words;
        this.ctx.request_layout();
    }

    /// Sets the [text alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    ///
    /// The runtime equivalent of [`with_text_alignment`](Self::with_text_alignment).
    pub fn set_text_alignment(this: &mut WidgetMut<'_, Self>, text_alignment: TextAlign) {
        this.widget.editor.set_alignment(text_alignment);

        this.ctx.request_layout();
    }

    /// Configures how this text area handles the user pressing Enter <kbd>↵</kbd>.
    pub fn set_insert_newline(this: &mut WidgetMut<'_, Self>, insert_newline: InsertNewline) {
        this.widget.insert_newline = insert_newline;
        this.ctx.request_accessibility_update();
    }

    /// Sets whether [hinting](https://en.wikipedia.org/wiki/Font_hinting) will be used for this text area.
    ///
    /// The runtime equivalent of [`with_hint`](Self::with_hint).
    /// For full documentation, see that method.
    pub fn set_hint(this: &mut WidgetMut<'_, Self>, hint: bool) {
        this.widget.hint = hint;
        this.ctx.request_paint_only();
    }

    /// Sets the selection to the given byte range.
    ///
    /// No-op if either index is not a char boundary.
    pub fn select_byte_range(this: &mut WidgetMut<'_, Self>, start: usize, end: usize) {
        let (fctx, lctx) = this.ctx.text_contexts();
        this.widget
            .editor
            .driver(fctx, lctx)
            .select_byte_range(start, end);
        this.ctx.request_render();
    }

    /// Sets the selection to the first instance of the given text.
    ///
    /// This is mostly useful for testing.
    ///
    /// No-op if the text isn't found.
    pub fn select_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        let Some(start) = this.widget.text().to_string().find(text) else {
            return;
        };
        let end = start + text.len();
        Self::select_byte_range(this, start, end);
    }
}

/// Text in a text area has been changed or submitted with enter.
#[derive(PartialEq, Debug)]
// TODO: Should this be two different structs?
pub enum TextAction {
    /// The text has been changed.
    Changed(String),
    /// The text has been submitted with the enter key.
    ///
    /// Whether this action gets emitted depends on the [`InsertNewline`] setting
    /// and with [`InsertNewline::OnShiftEnter`] also on if the shift key is pressed.
    Entered(String),
    // TODO: TextCursor changed, ImeChanged
}

// --- MARK: IMPL WIDGET
impl<const EDITABLE: bool> Widget for TextArea<EDITABLE> {
    type Action = TextAction;

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
        /// The time for a complete blink cycle, in milliseconds.
        /// For the first half (i.e. currently 0.5s) the cursor is shown, and for the second half,
        /// it is hidden.
        const CURSOR_BLINK_TIME: u64 = 1000;

        /// The timeout, in milliseconds, after which the cursor will stop blinking (i.e. stay
        /// solid).
        const CURSOR_BLINK_TIMEOUT: u64 = 10_000; // 10 seconds

        // TODO: These should be reading from the system settings, but we currently
        // aren't aware of a robust way to read that cross-platform.

        if ctx.is_window_focused() && ctx.is_focus_target() {
            if self.anim_elapsed < CURSOR_BLINK_TIMEOUT {
                let interval_ms = interval / 1_000_000; // ns to ms
                self.anim_prev_interval += interval_ms;
                self.anim_elapsed += interval_ms;

                if self.anim_prev_interval >= CURSOR_BLINK_TIME {
                    self.anim_prev_interval = self.anim_prev_interval.rem_euclid(CURSOR_BLINK_TIME);
                }

                // TODO: request timer here
                ctx.request_anim_frame();

                // Request paint only if changed.
                if self.anim_prev_interval < CURSOR_BLINK_TIME / 2 && !self.anim_cursor_visible {
                    self.anim_cursor_visible = true;
                    ctx.request_paint_only();
                } else if self.anim_prev_interval >= CURSOR_BLINK_TIME / 2
                    && self.anim_cursor_visible
                {
                    self.anim_cursor_visible = false;
                    ctx.request_paint_only();
                }
            } else if !self.anim_cursor_visible {
                // Request paint only if changed.
                self.anim_cursor_visible = true;
                ctx.request_paint_only();
            }
        }
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if self.editor.is_composing() {
            return;
        }

        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: None | Some(PointerButton::Primary),
                state,
                ..
            }) => {
                let cursor_pos = ctx.local_position(state.position);
                let (fctx, lctx) = ctx.text_contexts();
                let mut drv = self.editor.driver(fctx, lctx);
                match state.count {
                    2 => drv.select_word_at_point(cursor_pos.x as f32, cursor_pos.y as f32),
                    3 => {
                        drv.select_hard_line_at_point(cursor_pos.x as f32, cursor_pos.y as f32);
                    }
                    _ => {
                        if state.modifiers.shift() {
                            drv.shift_click_extension(cursor_pos.x as f32, cursor_pos.y as f32);
                        } else {
                            drv.move_to_point(cursor_pos.x as f32, cursor_pos.y as f32);
                        }
                    }
                }
                let new_generation = self.editor.generation();
                if new_generation != self.rendered_generation {
                    ctx.request_render();
                    ctx.set_ime_area(self.ime_area());
                    self.rendered_generation = new_generation;
                }
                ctx.request_focus();
                ctx.capture_pointer();
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    let cursor_pos = ctx.local_position(current.position);
                    let (fctx, lctx) = ctx.text_contexts();
                    self.editor
                        .driver(fctx, lctx)
                        .extend_selection_to_point(cursor_pos.x as f32, cursor_pos.y as f32);
                    let new_generation = self.editor.generation();
                    if new_generation != self.rendered_generation {
                        ctx.request_render();
                        ctx.set_ime_area(self.ime_area());
                        self.rendered_generation = new_generation;
                    }
                }
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        // Reset the blink animation.
        self.anim_prev_interval = 0;
        self.anim_elapsed = 0;
        ctx.request_anim_frame();

        match event {
            TextEvent::Keyboard(key_event) => {
                if key_event.state != KeyState::Down || self.editor.is_composing() {
                    return;
                }
                let (shift, action_mod) = (
                    key_event.modifiers.shift(),
                    if cfg!(target_os = "macos") {
                        key_event.modifiers.meta()
                    } else {
                        key_event.modifiers.ctrl()
                    },
                );
                let (fctx, lctx) = ctx.text_contexts();
                // Whether the text was changed.
                let mut edited = false;
                match &key_event.key {
                    // Cut
                    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                    Key::Character(x)
                        if EDITABLE && action_mod && x.as_str().eq_ignore_ascii_case("x") =>
                    {
                        if let Some(text) = self.editor.selected_text()
                            && !text.is_empty()
                        {
                            let text = text.to_string();
                            self.editor.driver(fctx, lctx).delete_selection();
                            edited = true;
                            ctx.set_clipboard(text);
                        }
                    }
                    // Copy
                    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                    Key::Character(c) if action_mod && c.as_str().eq_ignore_ascii_case("c") => {
                        if let Some(text) = self.editor.selected_text()
                            && !text.is_empty()
                        {
                            ctx.set_clipboard(text.to_string());
                        }
                    }
                    Key::Character(a) if action_mod && a.as_str().eq_ignore_ascii_case("a") => {
                        let mut drv = self.editor.driver(fctx, lctx);

                        if shift {
                            drv.collapse_selection();
                        } else {
                            drv.select_all();
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                drv.select_word_left();
                            } else {
                                drv.move_word_left();
                            }
                        } else if shift {
                            drv.select_left();
                        } else {
                            drv.move_left();
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                drv.select_word_right();
                            } else {
                                drv.move_word_right();
                            }
                        } else if shift {
                            drv.select_right();
                        } else {
                            drv.move_right();
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if shift {
                            drv.select_up();
                        } else {
                            drv.move_up();
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if shift {
                            drv.select_down();
                        } else {
                            drv.move_down();
                        }
                    }
                    Key::Named(NamedKey::Home) => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                drv.select_to_text_start();
                            } else {
                                drv.move_to_text_start();
                            }
                        } else if shift {
                            drv.select_to_line_start();
                        } else {
                            drv.move_to_line_start();
                        }
                    }
                    Key::Named(NamedKey::End) => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                drv.select_to_text_end();
                            } else {
                                drv.move_to_text_end();
                            }
                        } else if shift {
                            drv.select_to_line_end();
                        } else {
                            drv.move_to_line_end();
                        }
                    }
                    Key::Named(NamedKey::Delete) if EDITABLE => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if action_mod {
                            drv.delete_word();
                        } else {
                            drv.delete();
                        }

                        edited = true;
                    }
                    Key::Named(NamedKey::Backspace) if EDITABLE => {
                        let mut drv = self.editor.driver(fctx, lctx);
                        if action_mod {
                            drv.backdelete_word();
                        } else {
                            drv.backdelete();
                        }

                        edited = true;
                    }
                    Key::Character(sp) if EDITABLE && sp.as_str() == " " => {
                        self.editor
                            .driver(fctx, lctx)
                            .insert_or_replace_selection(" ");
                        edited = true;
                    }
                    Key::Named(NamedKey::Enter) => {
                        let insert_newline = match self.insert_newline {
                            InsertNewline::OnEnter => true,
                            InsertNewline::OnShiftEnter => shift,
                            InsertNewline::Never => false,
                        };
                        if insert_newline {
                            let (fctx, lctx) = ctx.text_contexts();
                            self.editor
                                .driver(fctx, lctx)
                                .insert_or_replace_selection("\n");
                            edited = true;
                        } else {
                            ctx.submit_action::<Self::Action>(TextAction::Entered(
                                self.text().to_string(),
                            ));
                        }
                    }

                    Key::Named(NamedKey::Tab) => {
                        // Intentionally do nothing so that tabbing from a TextInput/Prose works.
                        // Note that this doesn't allow input of the tab character; we need to be more clever here at some point
                        return;
                    }
                    Key::Character(text) if EDITABLE => {
                        self.editor
                            .driver(fctx, lctx)
                            .insert_or_replace_selection(text);
                        edited = true;
                    }
                    _ => {
                        // Do nothing, don't set as handled.
                        return;
                    }
                }
                ctx.set_handled();
                let new_generation = self.editor.generation();
                if new_generation != self.rendered_generation {
                    if edited {
                        ctx.submit_action::<Self::Action>(TextAction::Changed(
                            self.text().into_iter().collect(),
                        ));
                        ctx.request_layout();
                    } else {
                        ctx.request_render();
                        ctx.set_ime_area(self.ime_area());
                    }
                    self.rendered_generation = new_generation;
                }
            }

            TextEvent::WindowFocusChange(_) => {
                // To use a different selection color when unfocused.
                ctx.request_paint_only();
            }

            TextEvent::Ime(e) => {
                // TODO: Handle the cursor movement things from https://github.com/rust-windowing/winit/pull/3824
                let (fctx, lctx) = ctx.text_contexts();

                // Whether the returned text has changed.
                // We don't send a TextChanged when the preedit changes
                let mut edited = false;
                match e {
                    Ime::Disabled => {
                        self.editor.driver(fctx, lctx).clear_compose();
                    }
                    Ime::Preedit(text, cursor) => {
                        if text.is_empty() {
                            self.editor.driver(fctx, lctx).clear_compose();
                        } else {
                            self.editor.driver(fctx, lctx).set_compose(text, *cursor);
                            edited = true;
                        }
                    }
                    Ime::Commit(text) => {
                        self.editor
                            .driver(fctx, lctx)
                            .insert_or_replace_selection(text);
                        edited = true;
                    }
                    Ime::Enabled => {}
                }

                ctx.set_handled();
                if edited {
                    let text = self.text().into_iter().collect();
                    ctx.submit_action::<Self::Action>(TextAction::Changed(text));
                }

                let new_generation = self.editor.generation();
                if new_generation != self.rendered_generation {
                    ctx.request_layout();
                    self.rendered_generation = new_generation;
                }
            }

            TextEvent::ClipboardPaste(text) => {
                if EDITABLE {
                    let (fctx, lctx) = ctx.text_contexts();
                    self.editor
                        .driver(fctx, lctx)
                        .insert_or_replace_selection(text);

                    // TODO - Factor out with other branches
                    let new_generation = self.editor.generation();
                    if new_generation != self.rendered_generation {
                        ctx.submit_action::<Self::Action>(TextAction::Changed(
                            self.text().into_iter().collect(),
                        ));
                        ctx.request_layout();
                        self.rendered_generation = new_generation;
                    }
                }
            }
        }
    }

    fn accepts_focus(&self) -> bool {
        EDITABLE
    }

    fn accepts_text_input(&self) -> bool {
        EDITABLE
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if event.action == accesskit::Action::SetTextSelection {
            if self.editor.is_composing() {
                return;
            }

            if let Some(accesskit::ActionData::SetTextSelection(selection)) = &event.data {
                let (fctx, lctx) = ctx.text_contexts();
                self.editor
                    .driver(fctx, lctx)
                    .select_from_accesskit(selection);
                let new_generation = self.editor.generation();
                if new_generation != self.rendered_generation {
                    ctx.request_render();
                    ctx.set_ime_area(self.ime_area());
                    self.rendered_generation = new_generation;
                }
            }
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        CaretColor::prop_changed(ctx, property_type);
        ContentColor::prop_changed(ctx, property_type);
        DisabledContentColor::prop_changed(ctx, property_type);
        SelectionColor::prop_changed(ctx, property_type);
        UnfocusedSelectionColor::prop_changed(ctx, property_type);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                // HACK: We force the editor to relayout by pretending to edit the styles.
                //       We know that the lifecycle of dirty tracking in Parley's
                //       editor will need to change eventually anyway...
                let _ = self.editor.edit_styles();
                ctx.request_layout();
            }
            Update::FocusChanged(_) => {
                ctx.request_render();
            }
            Update::DisabledChanged(_) => {
                // We might need to use the disabled brush, and stop displaying the selection.
                ctx.request_render();
            }
            _ => {}
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        // TODO: The following max_advance calculation is very similar to Label widget's measure,
        //       so these could be more unified and share a single implementation.

        // Calculate the max advance for the inline axis, with None indicating unbounded.
        let max_advance = match self.word_wrap {
            true => {
                if axis == inline {
                    // Inline axis measurement ignores cross_length as a performance optimization.
                    // The search complexity of dealing with it is just too prohibitive.
                    // This is a common optimization also present on the web.
                    match len_req {
                        // Zero space will get us the length of longest unbreakable word
                        LenReq::MinContent => Some(0.),
                        // Unbounded space will get us the length of the unwrapped string
                        LenReq::MaxContent => None,
                        // Attempt to wrap according to the parent's request
                        LenReq::FitContent(space) => Some(space),
                    }
                } else {
                    // Block axis is dependant on the inline axis, so cross_length dominates.
                    // If there is no explicit cross_length present, we fall back to inline defaults.
                    match len_req {
                        // Fallback is inline axis MinContent
                        LenReq::MinContent => cross_length.or(Some(0.)),
                        // Fallback is inline axis MaxContent, even for FitContent, because
                        // as we don't have the inline space bound we'll consider it unbounded.
                        LenReq::MaxContent | LenReq::FitContent(_) => cross_length,
                    }
                }
            }
            // If we're never wrapping, then there's no max advance.
            false => None,
        }
        .map(|v| v as f32);

        let mut reset_max_advance = None;
        if self.last_max_advance != max_advance {
            reset_max_advance = Some(self.last_max_advance);
            self.editor.set_width(max_advance);
            self.last_max_advance = max_advance;
        }

        // TODO: PlainEditor::layout will do alignment and all,
        //       but that's potentially wasted work for measure.
        //       Should probably split up that PlainEditor method.

        let (fctx, lctx) = ctx.text_contexts();
        let layout = self.editor.layout(fctx, lctx);
        let text_width = max_advance.unwrap_or(layout.full_width());
        let text_size = Size::new(text_width.into(), layout.height().into());

        let length = text_size.get_coord(axis);

        // TODO: Remove this hack and do efficient side-effect free measurement with no alignment
        // HACK: Perform layout again with the old value so that speculative measure() calls
        //       won't affect paint() calls which expect the old layout() result to be present.
        if let Some(reset_max_advance) = reset_max_advance {
            self.editor.set_width(reset_max_advance);
            self.last_max_advance = reset_max_advance;
            self.editor.refresh_layout(fctx, lctx);
        }

        length
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        let inline_space = size.get_coord(inline) as f32;

        let max_advance = match self.word_wrap {
            true => Some(inline_space),
            false => None,
        };

        if self.last_max_advance != max_advance {
            self.editor.set_width(max_advance);
            self.last_max_advance = max_advance;
        }

        let new_generation = self.editor.generation();
        if new_generation != self.rendered_generation {
            self.rendered_generation = new_generation;
        }

        let (fctx, lctx) = ctx.text_contexts();
        self.editor.layout(fctx, lctx);

        ctx.set_ime_area(self.ime_area());
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let layout = if let Some(layout) = self.editor.try_layout() {
            layout
        } else {
            debug_panic!("Widget `layout` should have happened before paint");
            let (fctx, lctx) = ctx.text_contexts();
            // The `layout` method takes `&mut self`, so we get borrow-checker errors if we return it from this block.
            self.editor.refresh_layout(fctx, lctx);
            self.editor.try_layout().unwrap()
        };
        if ctx.is_focus_target() {
            let caret_color = props.get::<CaretColor>().color;
            let selection_color = if !ctx.is_window_focused()
                && let Some(us) = props.get_defined::<UnfocusedSelectionColor>()
            {
                us.0.color
            } else {
                props.get::<SelectionColor>().color
            };
            for (rect, _) in self.editor.selection_geometry().iter() {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    selection_color,
                    None,
                    &bounding_box_to_rect(*rect),
                );
            }
            if let Some(cursor) = self.editor.cursor_geometry(1.5)
                && self.anim_cursor_visible
                && ctx.is_window_focused()
            {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    caret_color,
                    None,
                    &bounding_box_to_rect(cursor),
                );
            };
        }

        let text_color = if ctx.is_disabled()
            && let Some(dc) = props.get_defined::<DisabledContentColor>()
        {
            &dc.0
        } else {
            props.get::<ContentColor>()
        };

        render_text(
            scene,
            Affine::IDENTITY,
            layout,
            &[text_color.color.into()],
            self.hint,
        );
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::Text
    }

    fn accessibility_role(&self) -> Role {
        if EDITABLE {
            match self.insert_newline {
                InsertNewline::OnShiftEnter | InsertNewline::OnEnter => Role::MultilineTextInput,
                _ => Role::TextInput,
            }
        } else {
            Role::Document
        }
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if !EDITABLE {
            node.set_read_only();
        }
        let updated =
            self.editor
                .try_accessibility(ctx.tree_update(), node, AccessCtx::next_node_id, 0., 0.);

        let Some(()) = updated else {
            // We always perform layout before accessibility, so this panic should be unreachable.
            debug_panic!("Could not generate accessibility nodes for text area");
            return;
        };
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("TextArea", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.editor.text().chars().take(100).collect())
    }
}

/// When to insert a newline in a text area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InsertNewline {
    /// Insert a newline when the user presses Enter.
    ///
    /// Note that if this is enabled, then the text area will never emit a [`TextAction::Entered`] event.
    OnEnter,
    /// Insert a newline when the user presses Shift+Enter.
    OnShiftEnter,
    /// Never insert a newline.
    #[default]
    Never,
}

// TODO: What other tests can we have? Some options:
// - Clicking in the right place changes the selection as expected?
// - Keyboard actions have expected results?

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use masonry_testing::TestHarnessParams;

    use super::*;
    use crate::core::{KeyboardEvent, Modifiers, NewWidget, PropertySet};
    use crate::kurbo::Size;
    use crate::palette;
    use crate::testing::TestHarness;
    use crate::theme::test_property_set;
    // Tests of alignment happen in Prose.

    #[test]
    fn edit_wordwrap() {
        let base_with_wrapping = {
            let area = NewWidget::new(
                TextArea::new_immutable("String which will wrap").with_word_wrap(true),
            );

            let mut harness =
                TestHarness::create_with_size(test_property_set(), area, Size::new(60.0, 40.0));

            harness.render()
        };

        {
            let area = NewWidget::new(
                TextArea::new_immutable("String which will wrap").with_word_wrap(false),
            );

            let mut harness =
                TestHarness::create_with_size(test_property_set(), area, Size::new(60.0, 40.0));

            let without_wrapping = harness.render();

            // Hack: If we are using `SKIP_RENDER_TESTS`, the output image is a 1x1 white pixel
            // This means that the not equal comparison won't work, so we skip it.
            // We should have a more principled solution here (or even better, get render tests working on windows)
            if !std::env::var("SKIP_RENDER_TESTS").is_ok_and(|it| !it.is_empty()) {
                // We don't use assert_eq because we don't want rich assert
                assert!(
                    base_with_wrapping != without_wrapping,
                    "Word wrapping being disabled should be obvious"
                );
            }

            harness.edit_root_widget(|mut area| {
                TextArea::set_word_wrap(&mut area, true);
            });

            let with_enabled_wrap = harness.render();

            // We don't use assert_eq because we don't want rich assert
            assert!(
                base_with_wrapping == with_enabled_wrap,
                "Updating the word wrap should correctly update"
            );
        };
    }

    #[test]
    fn edit_textarea() {
        let mut test_params = TestHarnessParams::default();
        test_params.window_size = Size::new(200.0, 20.0);

        let base_target = {
            let area = NewWidget::new_with_props(
                TextArea::new_immutable("Test string"),
                PropertySet::new().with(ContentColor::new(palette::css::AZURE)),
            );

            let mut harness = TestHarness::create_with(test_property_set(), area, test_params);

            harness.render()
        };

        {
            let area = NewWidget::new_with_props(
                TextArea::new_immutable("Different string"),
                PropertySet::new().with(ContentColor::new(palette::css::AZURE)),
            );

            let mut harness = TestHarness::create_with(test_property_set(), area, test_params);

            harness.edit_root_widget(|mut area| {
                TextArea::reset_text(&mut area, "Test string");
            });

            let with_updated_text = harness.render();

            // We don't use assert_eq because we don't want rich assert
            assert!(
                base_target == with_updated_text,
                "Updating the text should match with base text"
            );

            harness.edit_root_widget(|mut area| {
                area.insert_prop(ContentColor::new(palette::css::BROWN));
            });

            let with_updated_brush = harness.render();

            // Hack: If we are using `SKIP_RENDER_TESTS`, the output image is a 1x1 white pixel
            // This means that the not equal comparison won't work, so we skip it.
            if !std::env::var("SKIP_RENDER_TESTS").is_ok_and(|it| !it.is_empty()) {
                // We don't use assert_eq because we don't want rich assert
                assert!(
                    base_target != with_updated_brush,
                    "Updating the brush should have a visible change"
                );
            }
        };
    }

    #[test]
    fn insert_newline_behavior() {
        #[derive(Debug)]
        struct Scenario {
            insert_newline: InsertNewline,
            key: Key,
            modifiers: Modifiers,
            expect_text_entered_event: bool,
        }
        let scenarios = vec![
            Scenario {
                insert_newline: InsertNewline::OnEnter,
                key: Key::Named(NamedKey::Enter),
                modifiers: Modifiers::default(),
                expect_text_entered_event: false,
            },
            Scenario {
                insert_newline: InsertNewline::OnShiftEnter,
                key: Key::Named(NamedKey::Enter),
                modifiers: Modifiers::default(),
                expect_text_entered_event: true,
            },
            Scenario {
                insert_newline: InsertNewline::OnShiftEnter,
                key: Key::Named(NamedKey::Enter),
                modifiers: Modifiers::SHIFT,
                expect_text_entered_event: false,
            },
            Scenario {
                insert_newline: InsertNewline::Never,
                key: Key::Named(NamedKey::Enter),
                modifiers: Modifiers::default(),
                expect_text_entered_event: true,
            },
            Scenario {
                insert_newline: InsertNewline::Never,
                key: Key::Named(NamedKey::Enter),
                modifiers: Modifiers::SHIFT,
                expect_text_entered_event: true,
            },
        ];
        for scenario in scenarios {
            let area = NewWidget::new(
                TextArea::new_editable("hello world").with_insert_newline(scenario.insert_newline),
            );

            let mut harness = TestHarness::create(test_property_set(), area);
            let text_id = harness.root_id();

            harness.focus_on(Some(text_id));
            harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
                key: scenario.key,
                modifiers: scenario.modifiers,
                ..Default::default()
            }));

            let area = harness.root_widget();
            let text = area.text().to_string();
            let (action, widget_id) = harness.pop_action::<TextAction>().unwrap();
            assert_eq!(widget_id, text_id);

            // Check that only the one action was emitted so we don't miss an error case
            // where Entered _and_ Changed actions are emitted
            assert!(harness.pop_action_erased().is_none());

            if scenario.expect_text_entered_event {
                assert_eq!(action, TextAction::Entered("hello world".to_string()));
                assert_eq!(text, "hello world");
            } else {
                assert_eq!(action, TextAction::Changed("\nhello world".to_string()));
                assert_eq!(text, "\nhello world");
            }
        }
    }
}
