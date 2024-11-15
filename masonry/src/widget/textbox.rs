// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::mem::Discriminant;
use std::time::Instant;

use crate::text::{render_text, ActiveText, Generation, PlainEditor};
use accesskit::{Node, NodeId, Role};
use parley::layout::Alignment;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Insets, Point, Size, Stroke};
use vello::peniko::{BlendMode, Brush, Color, Fill};
use vello::Scene;
use winit::keyboard::{Key, NamedKey};

use crate::text::{ArcStr, BrushIndex, StyleProperty, StyleSet};
use crate::widget::{LineBreaking, WidgetMut};
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, CursorIcon, EventCtx, LayoutCtx, PaintCtx,
    PointerButton, PointerEvent, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetId,
};

const TEXTBOX_PADDING: f64 = 5.0;
/// HACK: A "margin" which is placed around the outside of all textboxes, ensuring that
/// they do not fill the entire width of the window.
///
/// This is added by making the width of the textbox be (twice) this amount less than
/// the space available, which is absolutely horrible.
///
/// In theory, this should be proper margin/padding in the parent widget, but that hasn't been
/// designed.
const TEXTBOX_X_MARGIN: f64 = 6.0;
/// The fallback minimum width for a textbox with infinite provided maximum width.
const INFINITE_TEXTBOX_WIDTH: f32 = 400.0;

/// The textbox widget is a widget which shows text which can be edited by the user
///
/// For immutable text [`Prose`](super::Prose) should be preferred
// TODO: RichTextBox 👀
pub struct Textbox {
    editor: PlainEditor<BrushIndex>,
    rendered_generation: Generation,

    pending_text: Option<ArcStr>,

    last_click_time: Option<Instant>,
    click_count: u32,

    // TODO: Support for links?
    //https://github.com/linebender/xilem/issues/360
    styles: StyleSet,
    /// Whether `styles` has been updated since `text_layout` was updated.
    ///
    /// If they have, the layout needs to be recreated.
    styles_changed: bool,

    line_break_mode: LineBreaking,
    alignment: Alignment,
    /// Whether the alignment has changed since the last layout, which would force a re-alignment.
    alignment_changed: bool,
    /// The value of `max_advance` when this layout was last calculated.
    ///
    /// If it has changed, we need to re-perform line-breaking.
    last_max_advance: Option<f32>,

    /// The brush for drawing this label's text.
    ///
    /// Requires a new paint if edited whilst `disabled_brush` is not being used.
    brush: Brush,
    /// The brush to use whilst this widget is disabled.
    ///
    /// When this is `None`, `brush` will be used.
    /// Requires a new paint if edited whilst this widget is disabled.
    disabled_brush: Option<Brush>,
    /// Whether to hint whilst drawing the text.
    ///
    /// Should be disabled whilst an animation involving this label is ongoing.
    // TODO: What classes of animations?
    hint: bool,
}

// --- MARK: BUILDERS ---
impl Textbox {
    pub fn new(text: impl Into<ArcStr>) -> Self {
        let editor = PlainEditor::default();
        Textbox {
            editor,
            rendered_generation: Generation::default(),
            pending_text: Some(text.into()),
            last_click_time: None,
            click_count: 0,
            styles: StyleSet::new(theme::TEXT_SIZE_NORMAL),
            styles_changed: true,
            line_break_mode: LineBreaking::WordWrap,
            alignment: Alignment::Start,
            alignment_changed: true,
            last_max_advance: None,
            brush: theme::TEXT_COLOR.into(),
            disabled_brush: Some(theme::DISABLED_TEXT_COLOR.into()),
            hint: true,
        }
    }

    /// Get the current text of this label.
    ///
    /// To update the text of an active label, use [`set_text`](Self::set_text).
    pub fn text(&self) -> &str {
        self.editor.text()
    }

    /// Set a style property for the new label.
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use `with_brush` instead.
    ///
    /// To set a style property on an active label, use [`insert_style`](Self::insert_style).
    pub fn with_style(mut self, property: impl Into<StyleProperty>) -> Self {
        self.insert_style_inner(property.into());
        self
    }

    /// Set a style property for the new label, returning the old value.
    ///
    /// Most users should prefer [`with_style`](Self::with_style) instead.
    pub fn try_with_style(
        mut self,
        property: impl Into<StyleProperty>,
    ) -> (Self, Option<StyleProperty>) {
        let old = self.insert_style_inner(property.into());
        (self, old)
    }

    /// Set how line breaks will be handled by this label.
    ///
    /// To modify this on an active label, use [`set_line_break_mode`](Self::set_line_break_mode).
    pub fn with_line_break_mode(mut self, line_break_mode: LineBreaking) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }

    /// Set the alignment of the text.
    ///
    /// Text alignment might have unexpected results when the label has no horizontal constraints.
    /// To modify this on an active label, use [`set_alignment`](Self::set_alignment).
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the brush used to paint this label.
    ///
    /// In most cases, this will be the text's color, but gradients and images are also supported.
    ///
    /// To modify this on an active label, use [`set_brush`](Self::set_brush).
    #[doc(alias = "with_color")]
    pub fn with_brush(mut self, brush: impl Into<Brush>) -> Self {
        self.brush = brush.into();
        self
    }

    /// Set the brush which will be used to paint this label whilst it is disabled.
    ///
    /// If this is `None`, the [normal brush](Self::with_brush) will be used.
    /// To modify this on an active label, use [`set_disabled_brush`](Self::set_disabled_brush).
    #[doc(alias = "with_color")]
    pub fn with_disabled_brush(mut self, disabled_brush: impl Into<Option<Brush>>) -> Self {
        self.disabled_brush = disabled_brush.into();
        self
    }

    /// Set whether [hinting](https://en.wikipedia.org/wiki/Font_hinting) will be used for this label.
    ///
    /// Hinting is a process where text is drawn "snapped" to pixel boundaries to improve fidelity.
    /// The default is true, i.e. hinting is enabled by default.
    ///
    /// This should be set to false if the label will be animated at creation.
    /// The kinds of relevant animations include changing variable font parameters,
    /// translating or scaling.
    /// Failing to do so will likely lead to an unpleasant shimmering effect, as different parts of the
    /// text "snap" at different times.
    ///
    /// To modify this on an active label, use [`set_hint`](Self::set_hint).
    // TODO: Should we tell each widget if smooth scrolling is ongoing so they can disable their hinting?
    // Alternatively, we should automate disabling hinting at the Vello layer when composing.
    pub fn with_hint(mut self, hint: bool) -> Self {
        self.hint = hint;
        self
    }

    /// Shared logic between `with_style` and `insert_style`
    fn insert_style_inner(&mut self, property: StyleProperty) -> Option<StyleProperty> {
        if let StyleProperty::Brush(idx @ BrushIndex(1..)) = &property {
            debug_panic!(
                "Can't set a non-zero brush index ({idx:?}) on a `Label`, as it only supports global styling."
            );
        }
        self.styles.insert(property)
    }
}

// --- MARK: WIDGETMUT ---
impl Textbox {
    // Note: These docs are lazy, but also have a decreased likelihood of going out of date.
    /// The runtime requivalent of [`with_style`](Self::with_style).
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`set_brush`](Self::set_brush) instead.
    pub fn insert_style(
        this: &mut WidgetMut<'_, Self>,
        property: impl Into<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.insert_style_inner(property.into());

        this.widget.styles_changed = true;
        this.ctx.request_layout();
        old
    }

    /// Keep only the styles for which `f` returns true.
    ///
    /// Styles which are removed return to Parley's default values.
    /// In most cases, these are the defaults for this widget.
    ///
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](parley::StyleProperty::FontSize).
    pub fn retain_styles(this: &mut WidgetMut<'_, Self>, f: impl FnMut(&StyleProperty) -> bool) {
        this.widget.styles.retain(f);

        this.widget.styles_changed = true;
        this.ctx.request_layout();
    }

    /// Remove the style with the discriminant `property`.
    ///
    /// To get the discriminant requires constructing a valid `StyleProperty` for the
    /// the desired property and passing it to [`core::mem::discriminant`].
    /// Getting this discriminant is usually possible in a `const` context.
    ///
    /// Styles which are removed return to Parley's default values.
    /// In most cases, these are the defaults for this widget.
    ///
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](parley::StyleProperty::FontSize).
    pub fn remove_style(
        this: &mut WidgetMut<'_, Self>,
        property: Discriminant<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.styles.remove(property);

        this.widget.styles_changed = true;
        this.ctx.request_layout();
        old
    }

    /// This is likely to be disruptive if the user is focused on this widget,
    /// and so should be avoided if possible.
    pub fn reset_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        this.widget.pending_text = Some(new_text.into());

        this.ctx.request_layout();
    }

    /// The runtime requivalent of [`with_line_break_mode`](Self::with_line_break_mode).
    pub fn set_line_break_mode(this: &mut WidgetMut<'_, Self>, line_break_mode: LineBreaking) {
        this.widget.line_break_mode = line_break_mode;
        // We don't need to set an internal invalidation, as `max_advance` is always recalculated
        this.ctx.request_layout();
    }

    /// The runtime requivalent of [`with_alignment`](Self::with_alignment).
    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: Alignment) {
        this.widget.alignment = alignment;

        this.widget.alignment_changed = true;
        this.ctx.request_layout();
    }

    #[doc(alias = "set_color")]
    /// The runtime requivalent of [`with_brush`](Self::with_brush).
    pub fn set_brush(this: &mut WidgetMut<'_, Self>, brush: impl Into<Brush>) {
        let brush = brush.into();
        this.widget.brush = brush;

        // We need to repaint unless the disabled brush is currently being used.
        if this.widget.disabled_brush.is_none() || this.ctx.is_disabled() {
            this.ctx.request_paint_only();
        }
    }

    /// The runtime requivalent of [`with_disabled_brush`](Self::with_disabled_brush).
    pub fn set_disabled_brush(this: &mut WidgetMut<'_, Self>, brush: impl Into<Option<Brush>>) {
        let brush = brush.into();
        this.widget.disabled_brush = brush;

        if this.ctx.is_disabled() {
            this.ctx.request_paint_only();
        }
    }

    /// The runtime requivalent of [`with_hint`](Self::with_hint).
    pub fn set_hint(this: &mut WidgetMut<'_, Self>, hint: bool) {
        this.widget.hint = hint;
        this.ctx.request_paint_only();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Textbox {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        if self.pending_text.is_some() {
            debug_panic!("`set_text` on `Prose` was called before an event started");
        }
        let window_origin = ctx.widget_state.window_origin();
        let inner_origin = Point::new(
            window_origin.x + TEXTBOX_X_MARGIN + TEXTBOX_PADDING,
            window_origin.y,
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
                    self.editor.transact(fctx, lctx, |txn| match click_count {
                        2 => txn.select_word_at_point(cursor_pos.x as f32, cursor_pos.y as f32),
                        3 => txn.select_line_at_point(cursor_pos.x as f32, cursor_pos.y as f32),
                        _ => txn.move_to_point(cursor_pos.x as f32, cursor_pos.y as f32),
                    });

                    let new_generation = self.editor.generation();
                    if new_generation != self.rendered_generation {
                        ctx.request_render();
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
                    self.editor.transact(fctx, lctx, |txn| {
                        txn.extend_selection_to_point(cursor_pos.x as f32, cursor_pos.y as f32);
                    });
                    let new_generation = self.editor.generation();
                    if new_generation != self.rendered_generation {
                        ctx.request_render();
                        self.rendered_generation = new_generation;
                    }
                }
            }
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        if self.pending_text.take().is_some() {
            debug_panic!("`set_text` on `Prose` was called before an event started");
        }
        match event {
            TextEvent::KeyboardKey(key_event, modifiers_state) => {
                if !key_event.state.is_pressed() {
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
                // Ideally we'd use key_without_modifiers, but that's broken
                match &key_event.logical_key {
                    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                    Key::Character(c) if action_mod && matches!(c.as_str(), "c" | "x" | "v") => {
                        // TODO: use clipboard_rs::{Clipboard, ClipboardContext};
                        match c.to_lowercase().as_str() {
                            "c" => {
                                if let ActiveText::Selection(_) = self.editor.active_text() {
                                    // let cb = ClipboardContext::new().unwrap();
                                    // cb.set_text(text.to_owned()).ok();
                                }
                            }
                            "x" => {
                                // if let ActiveText::Selection(text) = self.editor.active_text() {
                                //     let cb = ClipboardContext::new().unwrap();
                                //     cb.set_text(text.to_owned()).ok();
                                //     self.editor.transact(fcx, lcx, |txn| txn.delete_selection());
                                // }
                            }
                            "v" => {
                                // let cb = ClipboardContext::new().unwrap();
                                // let text = cb.get_text().unwrap_or_default();
                                // self.editor.transact(fcx, lcx, |txn| txn.insert_or_replace_selection(&text));
                            }
                            _ => (),
                        }
                    }
                    Key::Character(c) if action_mod && matches!(c.to_lowercase().as_str(), "a") => {
                        self.editor.transact(fctx, lctx, |txn| {
                            if shift {
                                txn.collapse_selection();
                            } else {
                                txn.select_all();
                            }
                        });
                    }
                    Key::Named(NamedKey::ArrowLeft) => self.editor.transact(fctx, lctx, |txn| {
                        if action_mod {
                            if shift {
                                txn.select_word_left();
                            } else {
                                txn.move_word_left();
                            }
                        } else if shift {
                            txn.select_left();
                        } else {
                            txn.move_left();
                        }
                    }),
                    Key::Named(NamedKey::ArrowRight) => self.editor.transact(fctx, lctx, |txn| {
                        if action_mod {
                            if shift {
                                txn.select_word_right();
                            } else {
                                txn.move_word_right();
                            }
                        } else if shift {
                            txn.select_right();
                        } else {
                            txn.move_right();
                        }
                    }),
                    Key::Named(NamedKey::ArrowUp) => self.editor.transact(fctx, lctx, |txn| {
                        if shift {
                            txn.select_up();
                        } else {
                            txn.move_up();
                        }
                    }),
                    Key::Named(NamedKey::ArrowDown) => self.editor.transact(fctx, lctx, |txn| {
                        if shift {
                            txn.select_down();
                        } else {
                            txn.move_down();
                        }
                    }),
                    Key::Named(NamedKey::Home) => self.editor.transact(fctx, lctx, |txn| {
                        if action_mod {
                            if shift {
                                txn.select_to_text_start();
                            } else {
                                txn.move_to_text_start();
                            }
                        } else if shift {
                            txn.select_to_line_start();
                        } else {
                            txn.move_to_line_start();
                        }
                    }),
                    Key::Named(NamedKey::End) => self.editor.transact(fctx, lctx, |txn| {
                        if action_mod {
                            if shift {
                                txn.select_to_text_end();
                            } else {
                                txn.move_to_text_end();
                            }
                        } else if shift {
                            txn.select_to_line_end();
                        } else {
                            txn.move_to_line_end();
                        }
                    }),
                    Key::Named(NamedKey::Delete) => self.editor.transact(fctx, lctx, |txn| {
                        if action_mod {
                            txn.delete_word();
                        } else {
                            txn.delete();
                        }
                    }),
                    Key::Named(NamedKey::Backspace) => self.editor.transact(fctx, lctx, |txn| {
                        if action_mod {
                            txn.backdelete_word();
                        } else {
                            txn.backdelete();
                        }
                    }),
                    Key::Named(NamedKey::Enter) => {
                        ctx.submit_action(crate::Action::TextEntered(self.text().to_string()));
                        return;
                        // let (fctx, lctx) = ctx.text_contexts();
                        // self.editor
                        //     .transact(fctx, lctx, |txn| txn.insert_or_replace_selection("\n"));
                    }
                    Key::Named(NamedKey::Space) => {
                        self.editor
                            .transact(fctx, lctx, |txn| txn.insert_or_replace_selection(" "));
                    }
                    _ => match &key_event.text {
                        Some(text) => {
                            self.editor
                                .transact(fctx, lctx, |txn| txn.insert_or_replace_selection(text));
                        }
                        None => {}
                    },
                }
                let new_generation = self.editor.generation();
                if new_generation != self.rendered_generation {
                    ctx.submit_action(crate::Action::TextChanged(self.text().to_string()));
                    // TODO: For all the non-text-input actions
                    ctx.request_layout();
                    self.rendered_generation = new_generation;
                }
            }
            // TODO: Set our highlighting colour to a lighter blue as window unfocused
            TextEvent::FocusChange(_) => {}
            TextEvent::Ime(e) => {
                // TODO: Handle the cursor movement things from https://github.com/rust-windowing/winit/pull/3824
                tracing::warn!(event = ?e, "Prose doesn't accept IME");
            }
            TextEvent::ModifierChange(_) => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        false
    }

    fn accepts_text_input(&self) -> bool {
        // TODO: Flip back to true.
        false
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if event.action == accesskit::Action::SetTextSelection {
            if let Some(accesskit::ActionData::SetTextSelection(selection)) = &event.data {
                let (fctx, lctx) = ctx.text_contexts();
                self.editor
                    .transact(fctx, lctx, |txn| txn.select_from_accesskit(selection));
            }
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
            Update::FocusChanged(false) => {
                ctx.request_render();
            }
            Update::FocusChanged(true) => {
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
        let (fctx, lctx) = ctx.text_contexts();
        let available_width = self.editor.transact(fctx, lctx, |txn| {
            if let Some(pending_text) = self.pending_text.take() {
                txn.select_to_text_start();
                txn.collapse_selection();
                txn.set_text(&pending_text);
            }
            let available_width = if bc.max().width.is_finite() {
                Some((bc.max().width - 2. * TEXTBOX_X_MARGIN - 2. * TEXTBOX_PADDING) as f32)
            } else {
                None
            };

            let max_advance = if self.line_break_mode == LineBreaking::WordWrap {
                available_width
            } else {
                None
            };
            if self.styles_changed {
                let style = self.styles.inner().values().cloned().collect();
                txn.set_default_style(style);
                self.styles_changed = false;
            }
            if max_advance != self.last_max_advance {
                txn.set_width(max_advance);
            }
            if self.alignment_changed {
                txn.set_alignment(self.alignment);
            }
            max_advance
        });
        let new_generation = self.editor.generation();
        if new_generation != self.rendered_generation {
            self.rendered_generation = new_generation;
        }

        let text_width = available_width
            .unwrap_or(self.editor.layout().full_width())
            .max(
                INFINITE_TEXTBOX_WIDTH.min(bc.max().width as f32)
                    - (2. * TEXTBOX_PADDING + 2. * TEXTBOX_X_MARGIN) as f32,
            );
        let text_size = Size::new(text_width.into(), self.editor.layout().height().into());

        let textbox_size = Size {
            height: text_size.height + 2. * TEXTBOX_PADDING,
            width: text_size.width + 2. * TEXTBOX_PADDING + 2. * TEXTBOX_X_MARGIN,
        };
        bc.constrain(textbox_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if self.line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        }

        let transform = Affine::translate((TEXTBOX_PADDING + TEXTBOX_X_MARGIN, TEXTBOX_PADDING));
        for rect in self.editor.selection_geometry().iter() {
            // TODO: If window not focused, use a different color
            // TODO: Make configurable
            scene.fill(Fill::NonZero, transform, Color::STEEL_BLUE, None, &rect);
        }

        if ctx.is_focused() {
            if let Some(cursor) = self.editor.selection_strong_geometry(1.5) {
                // TODO: Make configurable
                scene.fill(Fill::NonZero, transform, Color::WHITE, None, &cursor);
            };
            if let Some(cursor) = self.editor.selection_weak_geometry(1.5) {
                // TODO: Make configurable
                scene.fill(Fill::NonZero, transform, Color::LIGHT_GRAY, None, &cursor);
            };
        }

        let brush = if ctx.is_disabled() {
            self.disabled_brush
                .clone()
                .unwrap_or_else(|| self.brush.clone())
        } else {
            self.brush.clone()
        };
        // TODO: Is disabling hinting ever right for textbox?
        render_text(scene, transform, self.editor.layout(), &[brush], self.hint);

        if self.line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
        let size = ctx.size();
        let outline_rect = size
            .to_rect()
            .inset(Insets::uniform_xy(-TEXTBOX_X_MARGIN - 1.0, -1.0));
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::WHITE,
            None,
            &outline_rect,
        );
    }

    fn get_cursor(&self, _ctx: &QueryCtx, _pos: Point) -> CursorIcon {
        CursorIcon::Text
    }

    fn accessibility_role(&self) -> Role {
        Role::TextInput
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut Node) {
        self.editor.accessibility(
            ctx.tree_update,
            node,
            || NodeId::from(WidgetId::next()),
            TEXTBOX_X_MARGIN + TEXTBOX_PADDING,
            TEXTBOX_PADDING,
        );
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

// TODO - Add more tests
#[cfg(test)]
mod tests {
    use parley::{layout::Alignment, StyleProperty};
    use vello::kurbo::Size;

    use crate::{
        assert_render_snapshot,
        testing::TestHarness,
        widget::{CrossAxisAlignment, Flex, LineBreaking, Prose},
    };

    #[test]
    /// A wrapping prose's alignment should be respected, regardkess of
    /// its parent's alignment.
    fn prose_alignment_flex() {
        fn base_label() -> Prose {
            // Trailing whitespace is displayed when laying out prose.
            Prose::new("Hello  ")
                .with_style(StyleProperty::FontSize(10.0))
                .with_line_break_mode(LineBreaking::WordWrap)
        }
        let label1 = base_label().with_alignment(Alignment::Start);
        let label2 = base_label().with_alignment(Alignment::Middle);
        let label3 = base_label().with_alignment(Alignment::End);
        let label4 = base_label().with_alignment(Alignment::Start);
        let label5 = base_label().with_alignment(Alignment::Middle);
        let label6 = base_label().with_alignment(Alignment::End);
        let flex = Flex::column()
            .with_flex_child(label1, CrossAxisAlignment::Start)
            .with_flex_child(label2, CrossAxisAlignment::Start)
            .with_flex_child(label3, CrossAxisAlignment::Start)
            .with_flex_child(label4, CrossAxisAlignment::Center)
            .with_flex_child(label5, CrossAxisAlignment::Center)
            .with_flex_child(label6, CrossAxisAlignment::Center)
            .gap(0.0);

        let mut harness = TestHarness::create_with_size(flex, Size::new(80.0, 80.0));

        assert_render_snapshot!(harness, "prose_alignment_flex");
    }
}
