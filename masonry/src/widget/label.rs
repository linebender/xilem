// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label widget.

use std::mem::Discriminant;

use accesskit::{Node, Role, TextAlign};
use parley::layout::Alignment;
use parley::{Layout, LayoutAccessibility};
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::kurbo::{Affine, Size};
use vello::peniko::{BlendMode, Brush};
use vello::Scene;

use crate::text::{ArcStr, BrushIndex, StyleProperty, StyleSet};
use crate::widget::WidgetMut;
use crate::{
    theme, AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent,
    QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
};

/// Added padding between each horizontal edge of the widget
/// and the text in logical pixels.
const LABEL_X_PADDING: f64 = 2.0;

/// Options for handling lines that are too wide for the label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineBreaking {
    /// Lines are broken at word boundaries.
    WordWrap,
    /// Lines are truncated to the width of the label.
    Clip,
    /// Lines overflow the label.
    Overflow,
}

/// A widget displaying non-interactive text.
///
/// This is useful for creating interactive widgets which internally
/// need support for displaying text, such as a button.
pub struct Label {
    text_layout: Layout<()>,
    accessibility: LayoutAccessibility,

    text: ArcStr,
    styles: StyleSet,
    /// Whether `text` or `styles` has been updated since `text_layout` was created.
    ///
    /// If they have, the layout needs to be recreated.
    styles_changed: bool,

    line_break_mode: LineBreaking,
    alignment: Alignment,
    /// Whether the alignment has changed since the last layout, which would force a re-alignment.
    alignment_changed: bool,
    /// The value of max_advance when this layout was last calculated.
    ///
    /// If it has changed, we need to re-perform line-breaking.
    last_max_advance: Option<f32>,

    brush: Brush,
    /// The brush to use whilst this widget is disabled.
    ///
    /// When this is `None`, `brush` will be used.
    disabled_brush: Option<Brush>,
}

// --- MARK: BUILDERS ---
impl Label {
    /// Create a new label.
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self {
            text_layout: Layout::new(),
            accessibility: Default::default(),
            text: text.into(),
            styles: StyleSet::new(theme::TEXT_SIZE_NORMAL),
            styles_changed: true,
            line_break_mode: LineBreaking::Overflow,
            alignment: Alignment::Start,
            alignment_changed: true,
            last_max_advance: None,
            brush: theme::TEXT_COLOR.into(),
            disabled_brush: Some(theme::DISABLED_TEXT_COLOR.into()),
        }
    }

    /// Get the current of this label.
    pub fn text(&self) -> &ArcStr {
        &self.text
    }

    /// Set a style property for the new label.
    ///
    /// To add a style property on an existing label, use [`insert_style`](Self::insert_style).
    pub fn with_style(mut self, property: impl Into<StyleProperty>) -> Self {
        self.insert_style_inner(property.into());
        self
    }

    /// Set a style property, returning the old value.
    ///
    /// Most users should prefer [`with_style`](Self::with_style) instead.
    pub fn try_with_style(
        mut self,
        property: impl Into<StyleProperty>,
    ) -> (Self, Option<StyleProperty>) {
        let old = self.insert_style_inner(property.into());
        (self, old)
    }

    pub fn with_line_break_mode(mut self, line_break_mode: LineBreaking) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }

    /// Set the alignment of the text.
    ///
    /// Text alignment is ignored when the label has no horizontal constraints.
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Create a label with empty text.
    pub fn empty() -> Self {
        Self::new("")
    }

    /// Shared logic between `with_style` and `insert_style`
    fn insert_style_inner(&mut self, property: StyleProperty) -> Option<StyleProperty> {
        let property = property.into();
        if let StyleProperty::Brush(idx @ BrushIndex(1..)) = &property {
            debug_panic!(
                "Can't set a non-zero brush index ({idx:?}) on a `Label`, as it only supports global styling."
            );
        }
        self.styles.insert(property)
    }
}

// --- MARK: WIDGETMUT ---
impl Label {
    pub fn insert_style(
        this: &mut WidgetMut<'_, Self>,
        property: impl Into<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.insert_style_inner(property.into());

        this.widget.styles_changed = true;
        this.ctx.request_layout();
        old
    }

    pub fn retain_styles(this: &mut WidgetMut<'_, Self>, f: impl FnMut(&StyleProperty) -> bool) {
        this.widget.styles.retain(f);

        this.widget.styles_changed = true;
        this.ctx.request_layout();
    }

    pub fn remove_style(
        this: &mut WidgetMut<'_, Self>,
        property: Discriminant<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.styles.remove(property);

        this.widget.styles_changed = true;
        this.ctx.request_layout();
        old
    }

    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        this.widget.text = new_text.into();

        this.widget.styles_changed = true;
        this.ctx.request_layout();
    }

    pub fn set_alignment(this: &mut WidgetMut<'_, Self>, alignment: Alignment) {
        this.widget.alignment = alignment;

        this.widget.alignment_changed = true;
        this.ctx.request_layout();
    }

    #[doc(alias = "set_color")]
    pub fn set_brush(this: &mut WidgetMut<'_, Self>, brush: impl Into<Brush>) {
        let brush = brush.into();
        this.widget.brush = brush;

        // We need to repaint unless the disabled brush is currently being used.
        if this.widget.disabled_brush.is_none() || this.ctx.is_disabled() {
            this.ctx.request_paint_only();
        }
    }

    pub fn set_line_break_mode(this: &mut WidgetMut<'_, Self>, line_break_mode: LineBreaking) {
        this.widget.line_break_mode = line_break_mode;
        // We don't need to set an internal invalidation, as `max_advance` is always recalculated
        this.ctx.request_layout();
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Label {
    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
            Update::DisabledChanged(_) => {
                if self.disabled_brush.is_some() {
                    ctx.request_paint_only();
                }
            }
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let available_width = if bc.max().width.is_finite() {
            Some(bc.max().width as f32 - 2. * LABEL_X_PADDING as f32)
        } else {
            None
        };

        let max_advance = if self.line_break_mode == LineBreaking::WordWrap {
            available_width
        } else {
            None
        };
        let styles_changed = self.styles_changed;
        if self.styles_changed {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            // TODO(DJM): Build self.text_layout
            self.styles_changed = false;
        }

        if max_advance != self.last_max_advance || styles_changed {
            self.text_layout.break_all_lines(max_advance);
            self.last_max_advance = max_advance;
            self.alignment_changed = true;
        }

        let alignment_width = if self.alignment == Alignment::Start {
            self.text_layout.width()
        } else if let Some(width) = available_width {
            // We use the full available space to calculate text alignment and therefore
            // determine the widget's current width.
            //
            // As a special case, we don't do that if the alignment is to the start.
            // In theory, we should be passed down how our parent expects us to be aligned;
            // however that isn't currently handled.
            //
            // This does effectively mean that the widget takes up all the available space and
            // therefore doesn't play nicely with adjacent widgets unless `Start` alignment is used.
            //
            // The coherent way to have multiple items laid out on the same line and alignment is for them to
            // be inside the same text layout object "region".
            width
        } else {
            // TODO: Warn on the rising edge of entering this state for this widget?
            self.text_layout.width()
        };
        if self.alignment_changed {
            self.text_layout
                .align(Some(alignment_width), self.alignment);
        }
        let text_size = Size::new(alignment_width.into(), self.text_layout.height().into());

        let label_size = Size {
            height: text_size.height,
            width: text_size.width + 2. * LABEL_X_PADDING,
        };
        bc.constrain(label_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if self.line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        }

        // render_text();

        if self.line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, node: &mut Node) {
        self.accessibility.build_nodes(
            &self.text.as_ref(),
            layout,
            update,
            parent_node,
            next_node_id,
            x_offset,
            y_offset,
        );
        node.set_name(self.text.as_ref().to_string());
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Label", id = ctx.widget_id().trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text.to_string())
    }
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use parley::style::GenericFamily;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::theme::{PRIMARY_DARK, PRIMARY_LIGHT};
    use crate::widget::{CrossAxisAlignment, Flex, SizedBox};

    #[test]
    fn simple_label() {
        let label = Label::new("Hello");

        let mut harness = TestHarness::create(label);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello");
    }

    #[test]
    fn styled_label() {
        let label = Label::new("The quick brown fox jumps over the lazy dog")
            .with_text_brush(PRIMARY_LIGHT)
            .with_font_family(FontFamily::Generic(GenericFamily::Monospace))
            .with_text_size(20.0)
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_alignment(Alignment::Middle);

        let mut harness = TestHarness::create_with_size(label, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "styled_label");
    }

    #[test]
    /// A wrapping label's alignment should be respected, regardkess of
    /// its parent's alignment.
    fn label_alignment_flex() {
        fn base_label() -> Label {
            Label::new("Hello")
                .with_text_size(10.0)
                .with_line_break_mode(LineBreaking::WordWrap)
        }
        let label1 = base_label().with_text_alignment(Alignment::Start);
        let label2 = base_label().with_text_alignment(Alignment::Middle);
        let label3 = base_label().with_text_alignment(Alignment::End);
        let label4 = base_label().with_text_alignment(Alignment::Start);
        let label5 = base_label().with_text_alignment(Alignment::Middle);
        let label6 = base_label().with_text_alignment(Alignment::End);
        let flex = Flex::column()
            .with_flex_child(label1, CrossAxisAlignment::Start)
            .with_flex_child(label2, CrossAxisAlignment::Start)
            .with_flex_child(label3, CrossAxisAlignment::Start)
            .with_flex_child(label4, CrossAxisAlignment::Center)
            .with_flex_child(label5, CrossAxisAlignment::Center)
            .with_flex_child(label6, CrossAxisAlignment::Center)
            .gap(0.0);

        let mut harness = TestHarness::create_with_size(flex, Size::new(80.0, 80.0));

        assert_render_snapshot!(harness, "label_alignment_flex");
    }

    #[test]
    fn line_break_modes() {
        let widget = Flex::column()
            .with_flex_spacer(1.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::WordWrap),
                )
                .width(200.0),
            )
            .with_spacer(20.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::Clip),
                )
                .width(200.0),
            )
            .with_spacer(20.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::Overflow),
                )
                .width(200.0),
            )
            .with_flex_spacer(1.0);

        let mut harness = TestHarness::create(widget);

        assert_render_snapshot!(harness, "line_break_modes");
    }

    #[test]
    fn edit_label() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_text_brush(PRIMARY_LIGHT)
                .with_font_family(FontFamily::Generic(GenericFamily::Monospace))
                .with_text_size(20.0)
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_alignment(Alignment::Middle);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let label = Label::new("Hello world")
                .with_text_brush(PRIMARY_DARK)
                .with_text_size(40.0);

            let mut harness = TestHarness::create_with_size(label, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut label| {
                let mut label = label.downcast::<Label>();
                Label::set_text(&mut label, "The quick brown fox jumps over the lazy dog");
                Label::set_brush(&mut label, PRIMARY_LIGHT);
                Label::set_font_family(&mut label, FontFamily::Generic(GenericFamily::Monospace));
                Label::set_text_size(&mut label, 20.0);
                Label::set_line_break_mode(&mut label, LineBreaking::WordWrap);
                Label::set_alignment(&mut label, Alignment::Middle);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
