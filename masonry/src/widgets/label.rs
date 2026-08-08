// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::mem::Discriminant;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use tracing::{Span, trace_span};

use crate::core::{
    AccessCtx, ArcStr, BrushIndex, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, StyleSet, Update, UpdateCtx,
    UsesProperty, Widget, WidgetId, WidgetMut, render_text, set_accesskit_brush_properties,
};
use crate::imaging::Painter;
use crate::kurbo::{Affine, Axis, Point, Size};
use crate::layout::{AsUnit, LenReq, Length};
use crate::parley::LayoutAccessibility;
use crate::properties::{ContentColor, LineBreaking};
use crate::theme::default_text_styles;
use crate::util::debug_panic;
use crate::widgets::text_layout_cache::TextLayoutCache;
use crate::{TextAlign, theme};

/// A widget displaying non-interactive text.
///
/// This is useful for creating interactive widgets which internally
/// need support for displaying text, such as a button.
///
/// You can customize the look of this label with the
/// [`LineBreaking`] and [`ContentColor`] properties.
///
#[doc = concat!(
    "![Styled label](",
    include_doc_path!("screenshots/label_styled_label.png"),
    ")",
)]
pub struct Label {
    /// Cached layouts.
    layouts: TextLayoutCache,

    text: ArcStr,
    styles: StyleSet,
    text_alignment: TextAlign,

    /// Whether to hint whilst drawing the text.
    ///
    /// Should be disabled whilst an animation involving this label is ongoing.
    // TODO: What classes of animations?
    hint: bool,

    accessibility: LayoutAccessibility,
}

// --- MARK: BUILDERS
impl Label {
    /// Creates a new label with the given text.
    ///
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](crate::parley::StyleProperty::FontSize).
    pub fn new(text: impl Into<ArcStr>) -> Self {
        let mut styles = StyleSet::new(theme::TEXT_SIZE_NORMAL);
        default_text_styles(&mut styles);
        Self {
            layouts: TextLayoutCache::new(),
            text: text.into(),
            styles,
            text_alignment: TextAlign::Start,
            hint: true,
            accessibility: LayoutAccessibility::default(),
        }
    }

    /// Sets a style property for the new label.
    ///
    /// Setting [`StyleProperty::Brush`](crate::parley::StyleProperty::Brush) is not supported.
    /// Use the [`ContentColor`] property instead.
    ///
    /// To set a style property on an active label, use [`insert_style`](Self::insert_style).
    pub fn with_style(mut self, property: impl Into<StyleProperty>) -> Self {
        self.insert_style_inner(property.into());
        self
    }

    /// Sets a style property for the new label, returning the old value.
    ///
    /// Most users should prefer [`with_style`](Self::with_style) instead.
    pub fn try_with_style(
        mut self,
        property: impl Into<StyleProperty>,
    ) -> (Self, Option<StyleProperty>) {
        let old = self.insert_style_inner(property.into());
        (self, old)
    }

    /// Sets the alignment of the text.
    ///
    /// Text alignment might have unexpected results when the label has no horizontal constraints.
    /// To modify this on an active label, use [`set_text_alignment`](Self::set_text_alignment).
    pub fn with_text_alignment(mut self, text_alignment: TextAlign) -> Self {
        self.text_alignment = text_alignment;
        self
    }

    /// Sets whether [hinting](https://en.wikipedia.org/wiki/Font_hinting) will be used for this label.
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

// --- MARK: METHODS
impl Label {
    /// Returns a reference to the current text of this label.
    ///
    /// To update the text of an active label, use [`set_text`](Self::set_text).
    pub fn text(&self) -> &ArcStr {
        &self.text
    }
}

// --- MARK: WIDGETMUT
impl Label {
    // Note: These docs are lazy, but also have a decreased likelihood of going out of date.
    /// The runtime equivalent of [`with_style`](Self::with_style).
    ///
    /// Setting [`StyleProperty::Brush`](crate::parley::StyleProperty::Brush) is not supported.
    /// Use the [`ContentColor`] property instead.
    pub fn insert_style(
        this: &mut WidgetMut<'_, Self>,
        property: impl Into<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.insert_style_inner(property.into());

        this.widget.clear_cache();
        this.ctx.request_layout();
        old
    }

    /// Keeps only the styles for which `f` returns true.
    ///
    /// Styles which are removed return to Parley's default values.
    /// In most cases, these are the defaults for this widget.
    ///
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](crate::parley::StyleProperty::FontSize).
    pub fn retain_styles(this: &mut WidgetMut<'_, Self>, f: impl FnMut(&StyleProperty) -> bool) {
        this.widget.styles.retain(f);

        this.widget.clear_cache();
        this.ctx.request_layout();
    }

    /// Removes the style with the discriminant `property`.
    ///
    /// To get the discriminant requires constructing a valid `StyleProperty` for the
    /// the desired property and passing it to [`core::mem::discriminant`].
    /// Getting this discriminant is usually possible in a `const` context.
    ///
    /// Styles which are removed return to Parley's default values.
    /// In most cases, these are the defaults for this widget.
    ///
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](crate::parley::StyleProperty::FontSize).
    pub fn remove_style(
        this: &mut WidgetMut<'_, Self>,
        property: Discriminant<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.styles.remove(property);

        this.widget.clear_cache();
        this.ctx.request_layout();
        old
    }

    /// Replaces the text of this widget.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        this.widget.text = new_text.into();

        this.widget.clear_cache();
        this.ctx.request_layout();
    }

    /// The runtime equivalent of [`with_text_alignment`](Self::with_text_alignment).
    pub fn set_text_alignment(this: &mut WidgetMut<'_, Self>, text_alignment: TextAlign) {
        this.widget.text_alignment = text_alignment;
        this.ctx.request_layout();
    }

    /// The runtime equivalent of [`with_hint`](Self::with_hint).
    pub fn set_hint(this: &mut WidgetMut<'_, Self>, hint: bool) {
        this.widget.hint = hint;
        this.ctx.request_paint_only();
    }
}

impl Label {
    /// Clears the text layout cache.
    ///
    /// Call this whenever text, styles, or fonts have changed.
    fn clear_cache(&mut self) {
        self.layouts.clear();
    }
}

impl UsesProperty<ContentColor> for Label {}
impl UsesProperty<LineBreaking> for Label {}

// --- MARK: IMPL WIDGET
impl Widget for Label {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        LineBreaking::prop_changed(ctx, property_type);
        ContentColor::prop_changed(ctx, property_type);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.clear_cache();
                ctx.request_layout();
            }
            Update::DisabledChanged(_) => {
                ctx.request_render();
            }
            _ => {}
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        let cache = ctx.property_cache();
        let line_break_mode = props.get::<LineBreaking>(cache);

        // Calculate the max advance for the inline axis, with None indicating unbounded.
        let max_advance = match line_break_mode {
            LineBreaking::WordWrap => {
                if axis == inline {
                    // Inline axis measurement ignores cross_length as a performance optimization.
                    // The search complexity of dealing with it is just too prohibitive.
                    // This is a common optimization also present on the web.
                    match len_req {
                        // Zero space will get us the length of longest unbreakable word
                        LenReq::MinContent => Some(Length::ZERO),
                        // Unbounded space will get us the length of the unwrapped string
                        LenReq::MaxContent => None,
                        // Attempt to wrap according to the parent's request
                        LenReq::FitContent(space) => Some(space),
                    }
                } else {
                    // Block axis is dependent on the inline axis, so cross_length dominates.
                    // If there is no explicit cross_length present, we fall back to inline defaults.
                    match len_req {
                        // Fallback is inline axis MinContent
                        LenReq::MinContent => cross_length.or(Some(Length::ZERO)),
                        // Fallback is inline axis MaxContent, even for FitContent, because
                        // as we don't have the inline space bound we'll consider it unbounded.
                        LenReq::MaxContent | LenReq::FitContent(_) => cross_length,
                    }
                }
            }
            // If we're never wrapping, then there's no max advance.
            LineBreaking::Clip | LineBreaking::Overflow => None,
        }
        .map(|v| v.get() as f32);

        let (font_ctx, layout_ctx) = ctx.text_contexts();
        let layout_idx = self.layouts.build_and_break(
            font_ctx,
            layout_ctx,
            &self.text,
            &self.styles,
            max_advance,
        );
        let layout = self.layouts.get(layout_idx);

        let length = if axis == inline {
            layout.layout.width() // Inline length
        } else {
            layout.layout.height() // Block length
        };

        length.px()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        let cache = ctx.property_cache();
        let line_break_mode = props.get::<LineBreaking>(cache);

        let inline_space = size.get_coord(inline) as f32;

        let max_advance = match line_break_mode {
            LineBreaking::WordWrap => Some(inline_space),
            LineBreaking::Clip | LineBreaking::Overflow => None,
        };

        let (font_ctx, layout_ctx) = ctx.text_contexts();
        let layout_idx = self.layouts.build_and_break(
            font_ctx,
            layout_ctx,
            &self.text,
            &self.styles,
            max_advance,
        );
        self.layouts.set_active(layout_idx);
        let layout = self.layouts.active_mut();

        layout.align(self.text_alignment, inline_space);

        let line_count = layout.layout.len();
        if line_count > 0 {
            let line_first = layout.layout.get(0).unwrap();
            let line_last = layout.layout.get(line_count - 1).unwrap();
            let first_baseline = line_first.metrics().baseline as f64;
            let last_baseline = line_last.metrics().baseline as f64;
            ctx.set_baselines(first_baseline, last_baseline);
        } else {
            ctx.clear_baselines();
        }

        if *line_break_mode == LineBreaking::Clip {
            let border_box = size.to_rect() + ctx.border_box_insets();
            ctx.set_clip_path(border_box);
        } else {
            ctx.clear_clip_path();
        }
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let cache = ctx.property_cache();
        let text_color = props.get::<ContentColor>(cache);

        let layout = self.layouts.active();

        render_text(
            painter,
            Affine::IDENTITY,
            &layout.layout,
            &[text_color.color.into()],
            self.hint,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let text_origin_in_border_box_space = Point::ORIGIN + ctx.border_box_translation();

        let cache = ctx.property_cache();
        let text_color = props.get::<ContentColor>(cache);

        let layout = self.layouts.active();

        self.accessibility.build_nodes(
            self.text.as_ref(),
            &layout.layout,
            ctx.tree_update(),
            node,
            AccessCtx::next_node_id,
            text_origin_in_border_box_space.x,
            text_origin_in_border_box_space.y,
            |node, style| set_accesskit_brush_properties(node, style, &[text_color.color.into()]),
        );
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
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
    use super::*;
    use crate::core::{NewWidget, PropertySet};
    use crate::layout::{AsUnit, Dim};
    use crate::parley::style::GenericFamily;
    use crate::parley::{FontFamily, FontFamilyName, StyleProperty};
    use crate::properties::Dimensions;
    use crate::properties::Gap;
    use crate::properties::types::CrossAxisAlignment;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::{ACCENT_COLOR, test_property_set};
    use crate::widgets::{Flex, SizedBox};

    #[test]
    fn simple_label() {
        let label = Label::new("Hello").prepare();

        let mut harness = TestHarness::create_with_size(test_property_set(), label, (100, 40));

        assert_render_snapshot!(harness, "label_hello");
    }

    #[test]
    fn styled_label() {
        let label = Label::new("The quick brown fox jumps over the lazy dog")
            .with_style(FontFamily::Single(FontFamilyName::Generic(
                GenericFamily::Monospace,
            )))
            .with_style(StyleProperty::FontSize(20.0))
            .with_text_alignment(TextAlign::Center)
            .prepare()
            .with_props(
                PropertySet::new()
                    .with(ContentColor::new(ACCENT_COLOR))
                    .with(LineBreaking::WordWrap),
            );

        let mut harness = TestHarness::create_with_size(test_property_set(), label, (200, 200));

        assert_render_snapshot!(harness, "label_styled_label");
    }

    #[test]
    fn underline_label() {
        let label = Label::new("Emphasis")
            .with_style(StyleProperty::Underline(true))
            .prepare()
            .with_props(PropertySet::new().with(LineBreaking::WordWrap));

        let mut harness = TestHarness::create_with_size(test_property_set(), label, (100, 40));

        assert_render_snapshot!(harness, "label_underline_label");
    }
    #[test]
    fn strikethrough_label() {
        let label = Label::new("Tpyo")
            .with_style(StyleProperty::Strikethrough(true))
            .with_style(StyleProperty::StrikethroughSize(Some(4.)))
            .prepare()
            .with_props(PropertySet::new().with(LineBreaking::WordWrap));

        let mut harness = TestHarness::create_with_size(test_property_set(), label, (100, 40));

        assert_render_snapshot!(harness, "label_strikethrough_label");
    }

    #[test]
    /// A label's text alignment should be respected, regardless of
    /// its parent's alignment plans for it, if the label has stretched width.
    fn label_text_alignment_flex() {
        fn base_label(text_alignment: TextAlign) -> NewWidget<Label> {
            Label::new("Hello")
                .with_style(StyleProperty::FontSize(20.0))
                .with_text_alignment(text_alignment)
                .prepare()
                .with_props(Dimensions::width(Dim::Stretch))
        }
        let label1 = base_label(TextAlign::Start);
        let label2 = base_label(TextAlign::Center);
        let label3 = base_label(TextAlign::End);
        let label4 = base_label(TextAlign::Start);
        let label5 = base_label(TextAlign::Center);
        let label6 = base_label(TextAlign::End);
        let flex = Flex::column()
            .with(label1, CrossAxisAlignment::Start)
            .with(label2, CrossAxisAlignment::Start)
            .with(label3, CrossAxisAlignment::Start)
            .with(label4, CrossAxisAlignment::Center)
            .with(label5, CrossAxisAlignment::Center)
            .with(label6, CrossAxisAlignment::Center);
        let flex = NewWidget::new(flex).with_props(Gap::ZERO);

        let mut harness = TestHarness::create_with_size(test_property_set(), flex, (200, 200));

        assert_render_snapshot!(harness, "label_label_alignment_flex");
    }

    #[test]
    fn line_break_modes() {
        let widget = Flex::column()
            .with_spacer(1.0)
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .prepare()
                        .with_props(PropertySet::new().with(LineBreaking::WordWrap)),
                )
                .width(180.px())
                .prepare(),
            )
            .with_fixed_spacer(20.px())
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .prepare()
                        .with_props(PropertySet::new().with(LineBreaking::Clip)),
                )
                .width(180.px())
                .prepare(),
            )
            .with_fixed_spacer(20.px())
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .prepare()
                        .with_props(PropertySet::new().with(LineBreaking::Overflow)),
                )
                .width(180.px())
                .prepare(),
            )
            .with_spacer(1.0)
            .prepare();

        let mut harness = TestHarness::create_with_size(test_property_set(), widget, (200, 200));

        assert_render_snapshot!(harness, "label_line_break_modes");
    }

    #[test]
    fn edit_label() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_style(FontFamily::Single(FontFamilyName::Generic(
                    GenericFamily::Monospace,
                )))
                .with_style(StyleProperty::FontSize(20.0))
                .with_text_alignment(TextAlign::Center)
                .prepare()
                .with_props(
                    PropertySet::new()
                        .with(ContentColor::new(ACCENT_COLOR))
                        .with(LineBreaking::WordWrap),
                );

            let mut harness = TestHarness::create_with_size(test_property_set(), label, (50, 50));

            harness.render()
        };

        let image_2 = {
            let label = Label::new("Hello world")
                .with_style(StyleProperty::FontSize(40.0))
                .prepare();

            let mut harness = TestHarness::create_with_size(test_property_set(), label, (50, 50));

            harness.edit_root_widget(|mut label| {
                label.insert_prop(ContentColor::new(ACCENT_COLOR));
                label.insert_prop(LineBreaking::WordWrap);
                Label::set_text(&mut label, "The quick brown fox jumps over the lazy dog");
                Label::insert_style(
                    &mut label,
                    FontFamily::Single(FontFamilyName::Generic(GenericFamily::Monospace)),
                );
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
                Label::set_text_alignment(&mut label, TextAlign::Center);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
