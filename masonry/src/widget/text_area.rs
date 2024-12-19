// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

use std::mem::Discriminant;
use std::time::Instant;

use crate::kurbo::{Affine, Point, Size};
use crate::text::{default_styles, render_text};
use accesskit::{Node, NodeId, Role};
use parley::editor::{Generation, SplitString};
use parley::layout::Alignment;
use parley::PlainEditor;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Rect, Vec2};
use vello::peniko::{Brush, Color, Fill};
use vello::Scene;
use winit::keyboard::{Key, NamedKey};

use crate::text::{BrushIndex, StyleProperty};
use crate::widget::{Padding, WidgetMut};
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, CursorIcon, EventCtx, LayoutCtx, PaintCtx,
    PointerButton, PointerEvent, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId,
};

/// `TextArea` implements the core of interactive text.
///
/// It is used to implement [`Textbox`](super::Textbox) and [`Prose`](super::Prose).
/// It is rare that you will use a raw `TextArea` as a widget in your app; most users
/// should prefer one of those wrappers.
///
/// This ensures that the editable and read-only text have the same text selection and
/// copy/paste behaviour.
///
/// The `USER_EDITABLE` const generic parameter determines whether the text area's contents can be
/// edited by the user of the app.
/// This is true for `Textbox` and false for `Prose`.
///
/// This widget emits the following actions only when `USER_EDITABLE` is true:
///
/// - `TextEntered`, which is sent when the enter key is pressed
/// - `TextChanged`, which is sent whenever the text is changed
///
/// The exact semantics of how much horizontal space this widget takes up has not been determined.
/// In particular, this has consequences when the alignment is set.
// TODO: RichTextBox 👀
// TODO: Support for links - https://github.com/linebender/xilem/issues/360
pub struct TextArea<const USER_EDITABLE: bool> {
    // TODO: Placeholder text?
    /// The underlying `PlainEditor`, which provides a high-level interface for us to dispatch into.
    editor: PlainEditor<BrushIndex>,
    /// The generation of `editor` which we have rendered.
    ///
    /// TODO: Split into rendered and layout generation. This will make the `edited` mechanism in [`on_text_event`](Widget::on_text_event).
    rendered_generation: Generation,

    /// The time when this element was last clicked.
    ///
    /// Used to detect double/triple clicks.
    /// The long-term plan is for this to be provided by the platform (i.e. winit), as that has more context.
    last_click_time: Option<Instant>,
    /// How many clicks have occurred in this click sequence.
    click_count: u32,

    /// Whether to wrap words in this area.
    ///
    /// Note that if clipping is desired, that should be added by the parent widget.
    /// Can be set using [`set_word_wrap`](Self::set_word_wrap).
    word_wrap: bool,
    /// The amount of horizontal space available when [layout](Widget::layout) was
    /// last performed.
    ///
    /// If word wrapping is enabled, we use this for line breaking.
    /// We store this to avoid redoing work in layout and to set the
    /// width when `word_wrap` is re-enabled.
    last_available_width: Option<f32>,

    /// The brush for drawing this label's text.
    ///
    /// Requires a new paint if edited whilst `disabled_brush` is not being used.
    /// Can be set using [`set_brush`](Self::set_brush).
    brush: Brush,
    /// The brush to use whilst this widget is disabled.
    ///
    /// When this is `None`, `brush` will be used.
    /// Requires a new paint if edited whilst this widget is disabled.
    /// /// Can be set using [`set_disabled_brush`](Self::set_disabled_brush).
    disabled_brush: Option<Brush>,
    /// Whether to hint whilst drawing the text.
    ///
    /// Should be disabled whilst an animation involving this text is ongoing.
    /// Can be set using [`set_hint`](Self::set_hint).
    // TODO: What classes of animations? I.e does scrolling count?
    hint: bool,
    /// The amount of Padding inside this text area.
    ///
    /// This is generally expected to be set by the parent, but
    /// can also be overridden.
    /// Can be set using [`set_padding`](Self::set_padding).
    /// Immediate parent widgets should use [`with_padding_if_default`](Self::with_padding_if_default).
    padding: Padding,
}

// --- MARK: BUILDERS ---
impl TextArea<true> {
    /// Create a new `TextArea` which can be edited.
    ///
    /// Useful for creating a styled [Textbox](super::Textbox).
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new_editable(text: &str) -> Self {
        Self::new(text)
    }
}

impl TextArea<false> {
    /// Create a new `TextArea` which cannot be edited by the user.
    ///
    /// Useful for creating a styled [Prose](super::Prose).
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new_immutable(text: &str) -> Self {
        Self::new(text)
    }
}

impl<const EDITABLE: bool> TextArea<EDITABLE> {
    /// Create a new `TextArea` with the given text and default settings.
    ///
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new(text: &str) -> Self {
        let mut editor = PlainEditor::new(theme::TEXT_SIZE_NORMAL);
        default_styles(editor.edit_styles());
        editor.set_text(text);
        Self {
            editor,
            rendered_generation: Generation::default(),
            last_click_time: None,
            click_count: 0,
            word_wrap: true,
            last_available_width: None,
            brush: theme::TEXT_COLOR.into(),
            disabled_brush: Some(theme::DISABLED_TEXT_COLOR.into()),
            hint: true,
            // We use -0.0 to mark the default padding.
            // This allows parent views to overwrite it only if another source didn't configure it.
            padding: Padding::UNSET,
        }
    }

    /// Get the current text of this text area.
    ///
    /// To update the text of an active text area, use [`reset_text`](Self::reset_text).
    ///
    /// The return value is not just `&str` to handle IME preedits.
    pub fn text(&self) -> SplitString<'_> {
        self.editor.text()
    }

    /// Set a style property for the new text area.
    ///
    /// Style properties set by this method include [text size](parley::StyleProperty::FontSize),
    /// [font family](parley::StyleProperty::FontStack), [font weight](parley::StyleProperty::FontWeight),
    /// and [variable font parameters](parley::StyleProperty::FontVariations).
    /// The styles inserted here apply to the entire text; we currently do not
    /// support inline rich text.
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`set_brush`](Self::set_brush) instead.
    /// This is also not additive for [font stacks](parley::StyleProperty::FontStack), and
    /// instead overwrites any previous font stack.
    ///
    /// To set a style property on an active text area, use [`insert_style`](Self::insert_style).
    #[track_caller]
    pub fn with_style(mut self, property: impl Into<StyleProperty>) -> Self {
        self.insert_style_inner(property.into());
        self
    }

    /// Set a style property for the new text area, returning the old value.
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

    /// Set the [alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    ///
    /// Text alignment might have unexpected results when the text area has no horizontal constraints.
    ///
    /// To modify this on an active text area, use [`set_alignment`](Self::set_alignment).
    // TODO: Document behaviour based on provided minimum constraint?
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.editor.set_alignment(alignment);
        self
    }

    /// Set the brush used to paint the text in this text area.
    ///
    /// In most cases, this will be the text's color, but gradients and images are also supported.
    ///
    /// To modify this on an active text area, use [`set_brush`](Self::set_brush).
    #[doc(alias = "with_color")]
    pub fn with_brush(mut self, brush: impl Into<Brush>) -> Self {
        self.brush = brush.into();
        self
    }

    /// Set the brush which will be used to paint this text area whilst it is disabled.
    ///
    /// If this is `None`, the [normal brush](Self::with_brush) will be used.
    ///
    /// To modify this on an active text area, use [`set_disabled_brush`](Self::set_disabled_brush).
    #[doc(alias = "with_color")]
    pub fn with_disabled_brush(mut self, disabled_brush: impl Into<Option<Brush>>) -> Self {
        self.disabled_brush = disabled_brush.into();
        self
    }

    /// Set whether [hinting](https://en.wikipedia.org/wiki/Font_hinting) will be used for this text area.
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

    /// Set the padding around the text.
    ///
    /// This is the area outside the tight bound on the text where pointer events will be detected.
    ///
    /// To modify this on an active text area, use [`set_padding`](Self::set_padding).
    pub fn with_padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Adds `padding` unless [`with_padding`](Self::with_padding) was previously called.
    ///
    /// This is expected to be called when creating parent widgets.
    pub fn with_padding_if_default(mut self, padding: Padding) -> Self {
        if self.padding.is_unset() {
            self.padding = padding;
        }
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

// --- MARK: HELPERS ---
impl<const EDITABLE: bool> TextArea<EDITABLE> {
    /// Get the IME area from the editor, accounting for padding.
    ///
    /// This should only be called when the editor layout is available.
    fn ime_area(&self) -> Rect {
        debug_assert!(
            self.editor.try_layout().is_some(),
            "TextArea::ime_area should only be called when the editor layout is available"
        );
        let is_rtl = self
            .editor
            .try_layout()
            .map(|layout| layout.is_rtl())
            .unwrap_or(false);
        self.editor.ime_cursor_area() + Vec2::new(self.padding.get_left(is_rtl), self.padding.top)
    }
}

// --- MARK: WIDGETMUT ---
impl<const EDITABLE: bool> TextArea<EDITABLE> {
    /// Set font styling for an active text area.
    ///
    /// Style properties set by this method include [text size](parley::StyleProperty::FontSize),
    /// [font family](parley::StyleProperty::FontStack), [font weight](parley::StyleProperty::FontWeight),
    /// and [variable font parameters](parley::StyleProperty::FontVariations).
    /// The styles inserted here apply to the entire text; we currently do not
    /// support inline rich text.
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`set_brush`](Self::set_brush) instead.
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

    /// [Retain](std::vec::Vec::retain) only the styles for which `f` returns true.
    ///
    /// Styles which are removed return to Parley's default values.
    /// In most cases, these are the defaults for this widget.
    ///
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](parley::StyleProperty::FontSize).
    pub fn retain_styles(this: &mut WidgetMut<'_, Self>, f: impl FnMut(&StyleProperty) -> bool) {
        this.widget.editor.edit_styles().retain(f);

        this.ctx.request_layout();
    }

    /// Remove the style with the discriminant `property`.
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

    /// Set the text displayed in this widget.
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

        this.ctx.request_layout();
    }

    /// Control [word wrapping](https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap) for the text area.
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
        let width = if wrap_words {
            this.widget.last_available_width
        } else {
            None
        };
        this.widget.editor.set_width(width);
        this.ctx.request_layout();
    }

    /// Set the [alignment](https://en.wikipedia.org/wiki/Typographic_alignment) of the text.
    ///
    /// Text alignment might have unexpected results when the text area has no horizontal constraints.
    ///
    /// The runtime equivalent of [`with_alignment`](Self::with_alignment).
    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: Alignment) {
        this.widget.editor.set_alignment(alignment);

        this.ctx.request_layout();
    }

    #[doc(alias = "set_color")]
    /// Set the brush used to paint the text in this text area.
    ///
    /// In most cases, this will be the text's color, but gradients and images are also supported.
    ///
    /// The runtime equivalent of [`with_brush`](Self::with_brush).
    pub fn set_brush(this: &mut WidgetMut<'_, Self>, brush: impl Into<Brush>) {
        let brush = brush.into();
        this.widget.brush = brush;

        // We need to repaint unless the disabled brush is currently being used.
        if this.widget.disabled_brush.is_none() || !this.ctx.is_disabled() {
            this.ctx.request_paint_only();
        }
    }

    /// Set the brush used to paint this text area whilst it is disabled.
    ///
    /// If this is `None`, the [normal brush](Self::set_brush) will be used.
    ///
    /// The runtime equivalent of [`with_disabled_brush`](Self::with_disabled_brush).
    pub fn set_disabled_brush(this: &mut WidgetMut<'_, Self>, brush: impl Into<Option<Brush>>) {
        let brush = brush.into();
        this.widget.disabled_brush = brush;

        if this.ctx.is_disabled() {
            this.ctx.request_paint_only();
        }
    }

    /// Set whether [hinting](https://en.wikipedia.org/wiki/Font_hinting) will be used for this text area.
    ///
    /// The runtime equivalent of [`with_hint`](Self::with_hint).
    /// For full documentation, see that method.
    pub fn set_hint(this: &mut WidgetMut<'_, Self>, hint: bool) {
        this.widget.hint = hint;
        this.ctx.request_paint_only();
    }

    /// Set the padding around the text.
    ///
    /// This is the area outside the tight bound on the text where pointer events will be detected.
    ///
    /// The runtime equivalent of [`with_padding`](Self::with_padding).
    pub fn set_padding(this: &mut WidgetMut<'_, Self>, padding: impl Into<Padding>) {
        this.widget.padding = padding.into();
        // TODO: We could reset the width available to the editor here directly.
        // Determine whether there's any advantage to that
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET ---
impl<const EDITABLE: bool> Widget for TextArea<EDITABLE> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        if self.editor.is_composing() {
            return;
        }

        let window_origin = ctx.widget_state.window_origin();
        let (fctx, lctx) = ctx.text_contexts();
        let is_rtl = self.editor.layout(fctx, lctx).is_rtl();
        let inner_origin = Point::new(
            window_origin.x + self.padding.get_left(is_rtl),
            window_origin.y + self.padding.top,
        );
        match event {
            PointerEvent::PointerDown(button, state) => {
                if !ctx.is_disabled() && *button == PointerButton::Primary {
                    let now = Instant::now();
                    if let Some(last) = self.last_click_time.take() {
                        if now.duration_since(last).as_secs_f64() < 0.25 {
                            self.click_count = (self.click_count + 1) % 4;
                        } else {
                            self.click_count = 1;
                        }
                    } else {
                        self.click_count = 1;
                    }
                    self.last_click_time = Some(now);
                    let click_count = self.click_count;
                    let cursor_pos = Point::new(state.position.x, state.position.y) - inner_origin;
                    let (fctx, lctx) = ctx.text_contexts();
                    let mut drv = self.editor.driver(fctx, lctx);
                    match click_count {
                        2 => drv.select_word_at_point(cursor_pos.x as f32, cursor_pos.y as f32),
                        3 => drv.select_line_at_point(cursor_pos.x as f32, cursor_pos.y as f32),
                        _ => drv.move_to_point(cursor_pos.x as f32, cursor_pos.y as f32),
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
            }
            PointerEvent::PointerMove(state) => {
                if !ctx.is_disabled() && ctx.has_pointer_capture() {
                    let cursor_pos = Point::new(state.position.x, state.position.y) - inner_origin;
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

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        match event {
            TextEvent::KeyboardKey(key_event, modifiers_state) => {
                if !key_event.state.is_pressed() || self.editor.is_composing() {
                    return;
                }
                #[allow(unused)]
                let (shift, action_mod) = (
                    modifiers_state.shift_key(),
                    if cfg!(target_os = "macos") {
                        modifiers_state.super_key()
                    } else {
                        modifiers_state.control_key()
                    },
                );
                let (fctx, lctx) = ctx.text_contexts();
                // Whether the text was changed.
                let mut edited = false;
                // Ideally we'd use key_without_modifiers, but that's broken
                match &key_event.logical_key {
                    // Cut
                    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                    Key::Character(x)
                        if EDITABLE && action_mod && x.as_str().eq_ignore_ascii_case("x") =>
                    {
                        edited = true;
                        // TODO: use clipboard_rs::{Clipboard, ClipboardContext};
                        // if let Some(text) = self.editor.selected_text() {
                        //     let cb = ClipboardContext::new().unwrap();
                        //     cb.set_text(text.to_owned()).ok();
                        //     self.editor.drive(fcx, lcx, |drv| drv.delete_selection());
                        // }
                        // edited = true;
                    }
                    // Copy
                    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                    Key::Character(c) if action_mod && c.as_str().eq_ignore_ascii_case("c") => {
                        // TODO: use clipboard_rs::{Clipboard, ClipboardContext};
                        // if let Some(text) = self.editor.selected_text() {
                        //     let cb = ClipboardContext::new().unwrap();
                        //     cb.set_text(text.to_owned()).ok();
                        // }
                    }
                    // Paste
                    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                    Key::Character(v)
                        if EDITABLE && action_mod && v.as_str().eq_ignore_ascii_case("v") =>
                    {
                        edited = true;
                        // TODO: use clipboard_rs::{Clipboard, ClipboardContext};
                        // let cb = ClipboardContext::new().unwrap();
                        // let text = cb.get_text().unwrap_or_default();
                        // self.editor.drive(fcx, lcx, |drv| drv.insert_or_replace_selection(&text));
                        // edited = true;
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
                    Key::Named(NamedKey::Space) if EDITABLE => {
                        self.editor
                            .driver(fctx, lctx)
                            .insert_or_replace_selection(" ");
                        edited = true;
                    }
                    Key::Named(NamedKey::Enter) => {
                        // TODO: Multiline?
                        let multiline = false;
                        if multiline {
                            let (fctx, lctx) = ctx.text_contexts();
                            self.editor
                                .driver(fctx, lctx)
                                .insert_or_replace_selection("\n");
                            edited = true;
                        } else {
                            ctx.submit_action(crate::Action::TextEntered(self.text().to_string()));
                        }
                    }

                    Key::Named(NamedKey::Tab) => {
                        // Intentionally do nothing so that tabbing from a textbox/Prose works.
                        // Note that this doesn't allow input of the tab character; we need to be more clever here at some point
                        return;
                    }
                    _ if EDITABLE => match &key_event.text {
                        Some(text) => {
                            self.editor
                                .driver(fctx, lctx)
                                .insert_or_replace_selection(text);
                            edited = true;
                        }
                        None => {
                            // Do nothing, don't set as handled.
                            return;
                        }
                    },
                    _ => {
                        // Do nothing, don't set as handled.
                        return;
                    }
                }
                ctx.set_handled();
                let new_generation = self.editor.generation();
                if new_generation != self.rendered_generation {
                    if edited {
                        ctx.submit_action(crate::Action::TextChanged(
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
            // TODO: Set our highlighting colour to a lighter blue as window unfocused
            TextEvent::FocusChange(_) => {}
            TextEvent::Ime(e) => {
                // TODO: Handle the cursor movement things from https://github.com/rust-windowing/winit/pull/3824
                let (fctx, lctx) = ctx.text_contexts();

                // Whether the returned text has changed.
                // We don't send a TextChanged when the preedit changes
                let mut edited = false;
                match e {
                    winit::event::Ime::Disabled => {
                        self.editor.driver(fctx, lctx).clear_compose();
                    }
                    winit::event::Ime::Preedit(text, cursor) => {
                        if text.is_empty() {
                            self.editor.driver(fctx, lctx).clear_compose();
                        } else {
                            self.editor.driver(fctx, lctx).set_compose(text, *cursor);
                            edited = true;
                        }
                    }
                    winit::event::Ime::Commit(text) => {
                        self.editor
                            .driver(fctx, lctx)
                            .insert_or_replace_selection(text);
                        edited = true;
                    }
                    winit::event::Ime::Enabled => {}
                }

                ctx.set_handled();
                if edited {
                    let text = self.text().into_iter().collect();
                    ctx.submit_action(crate::Action::TextChanged(text));
                }

                let new_generation = self.editor.generation();
                if new_generation != self.rendered_generation {
                    ctx.request_layout();
                    self.rendered_generation = new_generation;
                }
            }
            TextEvent::ModifierChange(_) => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        EDITABLE
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if event.action == accesskit::Action::SetTextSelection {
            if self.editor.is_composing() {
                return;
            }

            if let Some(accesskit::ActionData::SetTextSelection(selection)) = &event.data {
                let (fctx, lctx) = ctx.text_contexts();
                self.editor
                    .driver(fctx, lctx)
                    .select_from_accesskit(selection);
            }
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
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

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        // Shrink constraints by padding inset
        let padding_size = Size::new(
            self.padding.leading + self.padding.trailing,
            self.padding.top + self.padding.bottom,
        );
        let sub_bc = bc.shrink(padding_size);

        let available_width = if bc.max().width.is_finite() {
            Some((sub_bc.max().width) as f32)
        } else {
            None
        };
        let max_advance = if self.word_wrap {
            available_width
        } else {
            None
        };
        if self.last_available_width != available_width && self.word_wrap {
            self.editor.set_width(max_advance);
        }
        self.last_available_width = available_width;
        // TODO: Use the minimum width in the bc for alignment

        let new_generation = self.editor.generation();
        if new_generation != self.rendered_generation {
            self.rendered_generation = new_generation;
        }

        let (fctx, lctx) = ctx.text_contexts();
        let layout = self.editor.layout(fctx, lctx);
        let text_width = max_advance.unwrap_or(layout.full_width());
        let text_size = Size::new(text_width.into(), layout.height().into());
        ctx.set_ime_area(self.ime_area());

        let area_size = Size {
            height: text_size.height + padding_size.height,
            width: text_size.width + padding_size.width,
        };
        bc.constrain(area_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let layout = if let Some(layout) = self.editor.try_layout() {
            layout
        } else {
            debug_panic!("Widget `layout` should have happened before paint");
            let (fctx, lctx) = ctx.text_contexts();
            // The `layout` method takes `&mut self`, so we get borrow-checker errors if we return it from this block.
            self.editor.refresh_layout(fctx, lctx);
            self.editor.try_layout().unwrap()
        };
        let is_rtl = layout.is_rtl();
        let origin = Vec2::new(self.padding.get_left(is_rtl), self.padding.top);
        let transform = Affine::translate(origin);
        if ctx.is_focused() {
            for rect in self.editor.selection_geometry().iter() {
                // TODO: If window not focused, use a different color
                // TODO: Make configurable
                scene.fill(Fill::NonZero, transform, Color::STEEL_BLUE, None, &rect);
            }
            if let Some(cursor) = self.editor.cursor_geometry(1.5) {
                // TODO: Make configurable
                scene.fill(Fill::NonZero, transform, Color::WHITE, None, &cursor);
            };
        }

        let brush = if ctx.is_disabled() {
            self.disabled_brush
                .clone()
                .unwrap_or_else(|| self.brush.clone())
        } else {
            self.brush.clone()
        };
        render_text(scene, transform, layout, &[brush], self.hint);
    }

    fn get_cursor(&self, _ctx: &QueryCtx, _pos: Point) -> CursorIcon {
        CursorIcon::Text
    }

    fn accessibility_role(&self) -> Role {
        if EDITABLE {
            Role::TextInput
            // TODO: Role::MultilineTextInput
        } else {
            Role::Document
        }
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut Node) {
        if !EDITABLE {
            node.set_read_only();
        }
        let (fctx, lctx) = ctx.text_contexts();
        let layout = self.editor.layout(fctx, lctx);
        let is_rtl = layout.is_rtl();
        let origin = ctx.window_origin();
        self.editor
            .try_accessibility(
                ctx.tree_update,
                node,
                || NodeId::from(WidgetId::next()),
                origin.x + self.padding.get_left(is_rtl),
                origin.y + self.padding.top,
            )
            .expect("We just performed a layout");
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Textbox", id = ctx.widget_id().trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.editor.text().chars().take(100).collect())
    }
}

// TODO: What other tests can we have? Some options:
// - Clicking in the right place changes the selection as expected?
// - Keyboard actions have expected results?

#[cfg(test)]
mod tests {
    use vello::{kurbo::Size, peniko::Color};

    use super::*;
    use crate::testing::TestHarness;
    // Tests of alignment happen in Prose.

    #[test]
    fn edit_wordwrap() {
        let base_with_wrapping = {
            let area = TextArea::new_immutable("String which will wrap").with_word_wrap(true);

            let mut harness = TestHarness::create_with_size(area, Size::new(60.0, 40.0));

            harness.render()
        };

        {
            let area = TextArea::new_immutable("String which will wrap").with_word_wrap(false);

            let mut harness = TestHarness::create_with_size(area, Size::new(60.0, 40.0));

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

            harness.edit_root_widget(|mut root| {
                let mut area = root.downcast::<TextArea<false>>();
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
        let base_target = {
            let area = TextArea::new_immutable("Test string").with_brush(Color::AZURE);

            let mut harness = TestHarness::create_with_size(area, Size::new(200.0, 20.0));

            harness.render()
        };

        {
            let area = TextArea::new_immutable("Different string").with_brush(Color::AZURE);

            let mut harness = TestHarness::create_with_size(area, Size::new(200.0, 20.0));

            harness.edit_root_widget(|mut root| {
                let mut area = root.downcast::<TextArea<false>>();
                TextArea::reset_text(&mut area, "Test string");
            });

            let with_updated_text = harness.render();

            // We don't use assert_eq because we don't want rich assert
            assert!(
                base_target == with_updated_text,
                "Updating the text should match with base text"
            );

            harness.edit_root_widget(|mut root| {
                let mut area = root.downcast::<TextArea<false>>();
                TextArea::set_brush(&mut area, Color::BROWN);
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
}
