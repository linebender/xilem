// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A label widget.

use std::any::TypeId;
use std::mem::Discriminant;

use accesskit::{Node, NodeId, Role};
use parley::{Layout, LayoutAccessibility};
use smallvec::SmallVec;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Affine, Size};
use vello::peniko::BlendMode;

use crate::core::{
    AccessCtx, ArcStr, BoxConstraints, BrushIndex, LayoutCtx, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, StyleProperty, StyleSet, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, render_text,
};
use crate::debug_panic;
use crate::properties::{DisabledTextColor, TextColor};
use crate::theme;
use crate::theme::default_text_styles;
use crate::{TextAlign, TextAlignOptions};

// TODO - Replace with Padding property.
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
///
#[doc = crate::include_screenshot!("label_styled_label.png", "Styled label.")]
pub struct Label {
    text_layout: Layout<BrushIndex>,
    accessibility: LayoutAccessibility,

    text: ArcStr,
    styles: StyleSet,
    /// Whether `text` or `styles` has been updated since `text_layout` was created.
    ///
    /// If they have, the layout needs to be recreated.
    styles_changed: bool,

    line_break_mode: LineBreaking,
    text_alignment: TextAlign,
    /// Whether the text alignment needs to be re-computed.
    needs_text_alignment: bool,
    /// How much width was available during last layout.
    last_available_width: Option<f32>,
    /// The value of `max_advance` when this layout was last calculated.
    ///
    /// If it has changed, we need to re-perform line-breaking.
    last_max_advance: Option<f32>,

    /// Should be disabled whilst an animation involving this label is ongoing.
    // TODO: What classes of animations?
    hint: bool,
}

// --- MARK: BUILDERS
impl Label {
    /// Create a new label with the given text.
    ///
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new(text: impl Into<ArcStr>) -> Self {
        let mut styles = StyleSet::new(theme::TEXT_SIZE_NORMAL);
        default_text_styles(&mut styles);
        Self {
            text_layout: Layout::new(),
            accessibility: LayoutAccessibility::default(),
            text: text.into(),
            styles,
            styles_changed: true,
            line_break_mode: LineBreaking::Overflow,
            text_alignment: TextAlign::Start,
            needs_text_alignment: true,
            last_available_width: None,
            last_max_advance: None,
            hint: true,
        }
    }

    /// Get the current text of this label.
    ///
    /// To update the text of an active label, use [`set_text`](Self::set_text).
    pub fn text(&self) -> &ArcStr {
        &self.text
    }

    /// Set a style property for the new label.
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`TextColor`] and [`DisabledTextColor`] properties instead.
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
    /// To modify this on an active label, use [`set_text_alignment`](Self::set_text_alignment).
    pub fn with_text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.text_alignment = text_alignment;
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
        if let StyleProperty::Brush(idx @ BrushIndex(1..))
        | StyleProperty::UnderlineBrush(Some(idx @ BrushIndex(1..)))
        | StyleProperty::StrikethroughBrush(Some(idx @ BrushIndex(1..))) = &property
        {
            debug_panic!(
                "Can't set a non-zero brush index ({idx:?}) on a `Label`, as it only supports global styling."
            );
        }
        self.styles.insert(property)
    }
}

// --- MARK: WIDGETMUT
impl Label {
    // Note: These docs are lazy, but also have a decreased likelihood of going out of date.
    /// The runtime equivalent of [`with_style`](Self::with_style).
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`TextColor`] and [`DisabledTextColor`] properties instead.
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

    /// Replace the text of this widget.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        this.widget.text = new_text.into();

        this.widget.styles_changed = true;
        this.ctx.request_layout();
    }

    /// The runtime equivalent of [`with_line_break_mode`](Self::with_line_break_mode).
    pub fn set_line_break_mode(this: &mut WidgetMut<'_, Self>, line_break_mode: LineBreaking) {
        this.widget.line_break_mode = line_break_mode;
        // We don't need to set an internal invalidation, as `max_advance` is always recalculated
        this.ctx.request_layout();
    }

    /// The runtime equivalent of [`with_text_alignment`](Self::with_text_alignment).
    pub fn set_text_alignment(this: &mut WidgetMut<'_, Self>, text_alignment: TextAlign) {
        this.widget.text_alignment = text_alignment;

        this.widget.needs_text_alignment = true;
        this.ctx.request_layout();
    }

    /// The runtime equivalent of [`with_hint`](Self::with_hint).
    pub fn set_hint(this: &mut WidgetMut<'_, Self>, hint: bool) {
        this.widget.hint = hint;
        this.ctx.request_paint_only();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Label {
    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        TextColor::prop_changed(ctx, property_type);
        DisabledTextColor::prop_changed(ctx, property_type);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let available_width = if bc.max().width.is_finite() {
            Some((bc.max().width as f32 - 2. * LABEL_X_PADDING as f32).max(0.))
        } else {
            None
        };
        if available_width != self.last_available_width {
            self.last_available_width = available_width;
            self.needs_text_alignment = true;
        }

        let max_advance = if self.line_break_mode == LineBreaking::WordWrap {
            available_width
        } else {
            None
        };
        let styles_changed = self.styles_changed;
        if self.styles_changed {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            // TODO: Should we use a different scale?
            let mut builder = layout_ctx.ranged_builder(font_ctx, &self.text, 1.0, true);
            for prop in self.styles.inner().values() {
                builder.push_default(prop.to_owned());
            }
            builder.build_into(&mut self.text_layout, &self.text);
            self.styles_changed = false;
        }

        if max_advance != self.last_max_advance || styles_changed {
            self.text_layout.break_all_lines(max_advance);
            self.last_max_advance = max_advance;
            self.needs_text_alignment = true;
        }

        let alignment_width = if self.text_alignment == TextAlign::Start {
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
        if self.needs_text_alignment {
            self.text_layout.align(
                Some(alignment_width),
                self.text_alignment,
                TextAlignOptions::default(),
            );
            self.needs_text_alignment = false;
        }
        let text_size = Size::new(alignment_width.into(), self.text_layout.height().into());

        let label_size = Size {
            height: text_size.height,
            width: text_size.width + 2. * LABEL_X_PADDING,
        };
        bc.constrain(label_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        if self.line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &clip_rect);
        }
        let transform = Affine::translate((LABEL_X_PADDING, 0.));

        let text_color = if ctx.is_disabled() {
            &props.get::<DisabledTextColor>().0
        } else {
            props.get::<TextColor>()
        };

        render_text(
            scene,
            transform,
            &self.text_layout,
            &[text_color.color.into()],
            self.hint,
        );

        if self.line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        self.accessibility.build_nodes(
            self.text.as_ref(),
            &self.text_layout,
            ctx.tree_update(),
            node,
            || NodeId::from(WidgetId::next()),
            LABEL_X_PADDING,
            0.0,
        );
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Label", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.text.to_string())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {

    use masonry_core::core::Properties;
    use masonry_testing::TestWidgetExt as _;
    use parley::style::GenericFamily;
    use parley::{FontFamily, StyleProperty};

    use super::*;
    use crate::assert_render_snapshot;
    use crate::testing::TestHarness;
    use crate::theme::{ACCENT_COLOR, default_property_set};
    use crate::widgets::{CrossAxisAlignment, Flex, SizedBox};

    #[test]
    fn simple_label() {
        let label = Label::new("Hello");

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(default_property_set(), label, window_size);

        assert_render_snapshot!(harness, "label_hello");
    }

    #[test]
    fn styled_label() {
        let label =  Label::new("The quick brown fox jumps over the lazy dog")
            .with_style(FontFamily::Generic(GenericFamily::Monospace))
            .with_style(StyleProperty::FontSize(20.0))
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_text_alignment(TextAlign::Center)
            .with_props(Properties::new().with(TextColor::new(ACCENT_COLOR)));

        let mut harness =
            TestHarness::create_with_size(default_property_set(), label, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "label_styled_label");
    }

    #[test]
    fn underline_label() {
        let label = Label::new("Emphasis")
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_style(StyleProperty::Underline(true));

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(default_property_set(), label, window_size);

        assert_render_snapshot!(harness, "label_underline_label");
    }
    #[test]
    fn strikethrough_label() {
        let label = Label::new("Tpyo")
            .with_line_break_mode(LineBreaking::WordWrap)
            .with_style(StyleProperty::Strikethrough(true))
            .with_style(StyleProperty::StrikethroughSize(Some(4.)));

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(default_property_set(), label, window_size);

        assert_render_snapshot!(harness, "label_strikethrough_label");
    }

    #[test]
    /// A wrapping label's text alignment should be respected, regardless of
    /// its parent's text alignment.
    fn label_text_alignment_flex() {
    fn base_label() -> Label {
        Label::new("Hello")
            .with_style(StyleProperty::FontSize(20.0))
            .with_line_break_mode(LineBreaking::WordWrap)
    }
    let label1 = base_label().with_text_alignment(TextAlign::Start);
    let label2 = base_label().with_text_alignment(TextAlign::Center);
    let label3 = base_label().with_text_alignment(TextAlign::End);
    let label4 = base_label().with_text_alignment(TextAlign::Start);
    let label5 = base_label().with_text_alignment(TextAlign::Center);
    let label6 = base_label().with_text_alignment(TextAlign::End);
    let flex = Flex::column()
        .with_flex_child(label1, CrossAxisAlignment::Start)
        .with_flex_child(label2, CrossAxisAlignment::Start)
        .with_flex_child(label3, CrossAxisAlignment::Start)
        // Text alignment start is "overwritten" by CrossAxisAlignment::Center.
        .with_flex_child(label4, CrossAxisAlignment::Center)
        .with_flex_child(label5, CrossAxisAlignment::Center)
        .with_flex_child(label6, CrossAxisAlignment::Center)
        .gap(0.0);

    let mut harness =
        TestHarness::create_with_size(default_property_set(), flex, Size::new(200.0, 200.0));

    assert_render_snapshot!(harness, "label_label_alignment_flex");
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
                .width(180.0),
            )
            .with_spacer(20.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::Clip),
                )
                .width(180.0),
            )
            .with_spacer(20.0)
            .with_child(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_line_break_mode(LineBreaking::Overflow),
                )
                .width(180.0),
            )
            .with_flex_spacer(1.0);

        let mut harness =
            TestHarness::create_with_size(default_property_set(), widget, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "label_line_break_modes");
    }

    #[test]
    fn edit_label() {
        let image_1 = {
            let label =  Label::new("The quick brown fox jumps over the lazy dog")
                .with_style(FontFamily::Generic(GenericFamily::Monospace))
                .with_style(StyleProperty::FontSize(20.0))
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_alignment(TextAlign::Center)
                .with_props(Properties::new().with(TextColor::new(ACCENT_COLOR)));

            let mut harness =
                TestHarness::create_with_size(default_property_set(), label, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let label = Label::new("Hello world").with_style(StyleProperty::FontSize(40.0));

            let mut harness =
                TestHarness::create_with_size(default_property_set(), label, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut label| {
                let mut label = label.downcast::<Label>();
                label.insert_prop(TextColor::new(ACCENT_COLOR));
                Label::set_text(&mut label, "The quick brown fox jumps over the lazy dog");
                Label::insert_style(&mut label, FontFamily::Generic(GenericFamily::Monospace));
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
                Label::set_line_break_mode(&mut label, LineBreaking::WordWrap);
                Label::set_text_alignment(&mut label, TextAlign::Center);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
