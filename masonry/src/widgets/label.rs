// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::mem::Discriminant;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use parley::{FontContext, Layout, LayoutAccessibility, LayoutContext};
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ArcStr, BrushIndex, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NoAction,
    PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, StyleSet, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, render_text,
};
use crate::kurbo::{Affine, Axis, Point, Size};
use crate::layout::LenReq;
use crate::peniko::{BlendMode, Fill};
use crate::properties::{ContentColor, DisabledContentColor, LineBreaking, Padding};
use crate::theme::default_text_styles;
use crate::util::debug_panic;
use crate::{TextAlign, TextAlignOptions, theme};

/// A widget displaying non-interactive text.
///
/// This is useful for creating interactive widgets which internally
/// need support for displaying text, such as a button.
///
/// You can customize the look of this label with the
/// [`Padding`], [`LineBreaking`], [`ContentColor`] and [`DisabledContentColor`] properties.
///
#[doc = concat!(
    "![Styled label](",
    include_doc_path!("screenshots/label_styled_label.png"),
    ")",
)]
pub struct Label {
    text_layout: TextLayout,
    measure_text_layout: TextLayout,
    accessibility: LayoutAccessibility,

    text: ArcStr,
    styles: StyleSet,
    /// Whether `text` or `styles` has been updated since `text_layout` was created.
    ///
    /// If they have, the layout needs to be recreated.
    styles_changed: bool,

    text_alignment: TextAlign,

    /// The amount of available inline space during last layout.
    last_inline_space: f32,

    /// Whether to hint whilst drawing the text.
    ///
    /// Should be disabled whilst an animation involving this label is ongoing.
    // TODO: What classes of animations?
    hint: bool,
}

struct TextLayout {
    layout: Layout<BrushIndex>,

    /// Whether the text alignment needs to be re-computed.
    needs_text_alignment: bool,

    /// The value of `max_advance` when this layout was last calculated.
    ///
    /// If it has changed, we need to re-perform line-breaking.
    last_max_advance: Option<f32>,
}

impl TextLayout {
    fn new() -> Self {
        Self {
            layout: Layout::new(),
            needs_text_alignment: true,
            last_max_advance: None,
        }
    }
}

// --- MARK: BUILDERS
impl Label {
    /// Creates a new label with the given text.
    ///
    // This is written out fully to appease rust-analyzer; StyleProperty is imported but not recognised.
    /// To change the font size, use `with_style`, setting [`StyleProperty::FontSize`](parley::StyleProperty::FontSize).
    pub fn new(text: impl Into<ArcStr>) -> Self {
        let mut styles = StyleSet::new(theme::TEXT_SIZE_NORMAL);
        default_text_styles(&mut styles);
        Self {
            text_layout: TextLayout::new(),
            measure_text_layout: TextLayout::new(),
            accessibility: LayoutAccessibility::default(),
            text: text.into(),
            styles,
            styles_changed: true,
            text_alignment: TextAlign::Start,
            last_inline_space: 0.,
            hint: true,
        }
    }

    /// Sets a style property for the new label.
    ///
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`ContentColor`] and [`DisabledContentColor`] properties instead.
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
    /// Setting [`StyleProperty::Brush`](parley::StyleProperty::Brush) is not supported.
    /// Use [`ContentColor`] and [`DisabledContentColor`] properties instead.
    pub fn insert_style(
        this: &mut WidgetMut<'_, Self>,
        property: impl Into<StyleProperty>,
    ) -> Option<StyleProperty> {
        let old = this.widget.insert_style_inner(property.into());

        this.widget.styles_changed = true;
        this.ctx.request_layout();
        old
    }

    /// Keeps only the styles for which `f` returns true.
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

    /// Removes the style with the discriminant `property`.
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

    /// Replaces the text of this widget.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        this.widget.text = new_text.into();

        this.widget.styles_changed = true;
        this.ctx.request_layout();
    }

    /// The runtime equivalent of [`with_text_alignment`](Self::with_text_alignment).
    pub fn set_text_alignment(this: &mut WidgetMut<'_, Self>, text_alignment: TextAlign) {
        this.widget.text_alignment = text_alignment;

        this.widget.text_layout.needs_text_alignment = true;
        this.widget.measure_text_layout.needs_text_alignment = true;
        this.ctx.request_layout();
    }

    /// The runtime equivalent of [`with_hint`](Self::with_hint).
    pub fn set_hint(this: &mut WidgetMut<'_, Self>, hint: bool) {
        this.widget.hint = hint;
        this.ctx.request_paint_only();
    }
}

impl Label {
    /// Builds the text layout and breaks the text into lines.
    fn build_and_break(
        &mut self,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<BrushIndex>,
        fonts_changed: bool,
        max_advance: Option<f32>,
        commit: bool,
    ) {
        // TODO: Rewrite this abomination in a far more efficient way.
        //       There should be a simple LRU cache like MeasurementCache,
        //       with one committed entry as immutable and undeletable.

        // TODO: Don't trigger style change multiple times per layout pass for font changes,
        //       by storing some marker that states we've already dealt with it this pass.
        let styles_changed = self.styles_changed || fonts_changed;
        if styles_changed {
            {
                // TODO: Should we use a different scale?
                // See https://github.com/linebender/xilem/issues/1264
                let mut builder = layout_ctx.ranged_builder(font_ctx, &self.text, 1.0, true);
                for prop in self.styles.inner().values() {
                    builder.push_default(prop.to_owned());
                }
                builder.build_into(&mut self.measure_text_layout.layout, &self.text);
            }
            if commit {
                // TODO: Should we use a different scale?
                // See https://github.com/linebender/xilem/issues/1264
                let mut builder = layout_ctx.ranged_builder(font_ctx, &self.text, 1.0, true);
                for prop in self.styles.inner().values() {
                    builder.push_default(prop.to_owned());
                }
                builder.build_into(&mut self.text_layout.layout, &self.text);
                self.styles_changed = false;
            }
        }

        {
            if styles_changed || max_advance != self.measure_text_layout.last_max_advance {
                self.measure_text_layout.layout.break_all_lines(max_advance);
                self.measure_text_layout.last_max_advance = max_advance;
                self.measure_text_layout.needs_text_alignment = true;
            }
        }
        if commit && (styles_changed || max_advance != self.text_layout.last_max_advance) {
            self.text_layout.layout.break_all_lines(max_advance);
            self.text_layout.last_max_advance = max_advance;
            self.text_layout.needs_text_alignment = true;
        }
    }
}

impl HasProperty<ContentColor> for Label {}
impl HasProperty<DisabledContentColor> for Label {}
impl HasProperty<LineBreaking> for Label {}

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
        DisabledContentColor::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::DisabledChanged(_) => {
                ctx.request_paint_only();
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
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        let padding = props.get::<Padding>();
        let line_break_mode = props.get::<LineBreaking>();

        let padding_length = padding.length(axis).dp(scale);

        // Calculate the max advance for the inline axis, with None indicating unbounded.
        let max_advance = match line_break_mode {
            LineBreaking::WordWrap => {
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
                        LenReq::FitContent(space) => Some((space - padding_length).max(0.)),
                    }
                } else {
                    // Block axis is dependant on the inline axis, so cross_length dominates.
                    // If there is no explicit cross_length present, we fall back to inline defaults.
                    let cross_space = cross_length.map(|cross_length| {
                        let cross = axis.cross();
                        let cross_padding_length = padding.length(cross).dp(scale);
                        cross_length - cross_padding_length
                    });
                    match len_req {
                        // Fallback is inline axis MinContent
                        LenReq::MinContent => cross_space.or(Some(0.)),
                        // Fallback is inline axis MaxContent, even for FitContent, because
                        // as we don't have the inline space bound we'll consider it unbounded.
                        LenReq::MaxContent | LenReq::FitContent(_) => cross_space,
                    }
                }
            }
            // If we're never wrapping, then there's no max advance.
            LineBreaking::Clip | LineBreaking::Overflow => None,
        }
        .map(|v| v as f32);

        let fonts_changed = ctx.fonts_changed();
        let (font_ctx, layout_ctx) = ctx.text_contexts();
        self.build_and_break(font_ctx, layout_ctx, fonts_changed, max_advance, false);

        let length = if axis == inline {
            self.measure_text_layout.layout.width() // Inline length
        } else {
            self.measure_text_layout.layout.height() // Block length
        };

        length as f64 + padding_length
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        let padding = props.get::<Padding>();
        let line_break_mode = props.get::<LineBreaking>();

        let space = padding.size_down(size, scale);
        let inline_space = space.get_coord(inline) as f32;

        if self.last_inline_space != inline_space {
            self.last_inline_space = inline_space;
            self.text_layout.needs_text_alignment = true;
        }

        let max_advance = match line_break_mode {
            LineBreaking::WordWrap => Some(inline_space),
            LineBreaking::Clip | LineBreaking::Overflow => None,
        };

        let fonts_changed = ctx.fonts_changed();
        let (font_ctx, layout_ctx) = ctx.text_contexts();
        self.build_and_break(font_ctx, layout_ctx, fonts_changed, max_advance, true);

        if self.text_layout.needs_text_alignment {
            self.text_layout.layout.align(
                Some(inline_space),
                self.text_alignment,
                TextAlignOptions::default(),
            );
            self.text_layout.needs_text_alignment = false;
        }

        let baseline = 0.; // TODO: Use actual baseline, at least for single line text
        let baseline = padding.baseline_up(baseline, scale);
        ctx.set_baseline_offset(baseline);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let padding = *props.get::<Padding>();
        let line_break_mode = *props.get::<LineBreaking>();

        if line_break_mode == LineBreaking::Clip {
            let clip_rect = ctx.size().to_rect();
            scene.push_layer(
                Fill::NonZero,
                BlendMode::default(),
                1.,
                Affine::IDENTITY,
                &clip_rect,
            );
        }
        let text_origin = padding.origin_down(Point::ZERO, scale).to_vec2();
        let transform = Affine::translate(text_origin);

        let text_color = if ctx.is_disabled() {
            &props.get::<DisabledContentColor>().0
        } else {
            props.get::<ContentColor>()
        };

        render_text(
            scene,
            transform,
            &self.text_layout.layout,
            &[text_color.color.into()],
            self.hint,
        );

        if line_break_mode == LineBreaking::Clip {
            scene.pop_layer();
        }
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
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let padding = *props.get::<Padding>();

        let text_origin = padding.origin_down(Point::ZERO, scale).to_vec2();
        self.accessibility.build_nodes(
            self.text.as_ref(),
            &self.text_layout.layout,
            ctx.tree_update(),
            node,
            AccessCtx::next_node_id,
            text_origin.x,
            text_origin.y,
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
    use parley::style::GenericFamily;
    use parley::{FontFamily, StyleProperty};

    use super::*;
    use crate::core::{NewWidget, Properties};
    use crate::layout::{AsUnit, Dim};
    use crate::properties::Dimensions;
    use crate::properties::Gap;
    use crate::properties::types::CrossAxisAlignment;
    use crate::testing::{TestHarness, assert_render_snapshot};
    use crate::theme::{ACCENT_COLOR, test_property_set};
    use crate::widgets::{Flex, SizedBox};

    #[test]
    fn simple_label() {
        let label = Label::new("Hello").with_auto_id();

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), label, window_size);

        assert_render_snapshot!(harness, "label_hello");
    }

    #[test]
    fn styled_label() {
        let label = Label::new("The quick brown fox jumps over the lazy dog")
            .with_style(FontFamily::Generic(GenericFamily::Monospace))
            .with_style(StyleProperty::FontSize(20.0))
            .with_text_alignment(TextAlign::Center)
            .with_props(
                Properties::new()
                    .with(ContentColor::new(ACCENT_COLOR))
                    .with(LineBreaking::WordWrap),
            );

        let mut harness =
            TestHarness::create_with_size(test_property_set(), label, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "label_styled_label");
    }

    #[test]
    fn underline_label() {
        let label = Label::new("Emphasis")
            .with_style(StyleProperty::Underline(true))
            .with_props(Properties::new().with(LineBreaking::WordWrap));

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), label, window_size);

        assert_render_snapshot!(harness, "label_underline_label");
    }
    #[test]
    fn strikethrough_label() {
        let label = Label::new("Tpyo")
            .with_style(StyleProperty::Strikethrough(true))
            .with_style(StyleProperty::StrikethroughSize(Some(4.)))
            .with_props(Properties::new().with(LineBreaking::WordWrap));

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), label, window_size);

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
        let flex = NewWidget::new_with_props(flex, Gap::ZERO);

        let mut harness =
            TestHarness::create_with_size(test_property_set(), flex, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "label_label_alignment_flex");
    }

    #[test]
    fn line_break_modes() {
        let widget = Flex::column()
            .with_spacer(1.0)
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_props(Properties::new().with(LineBreaking::WordWrap)),
                )
                .width(180.px())
                .with_auto_id(),
            )
            .with_fixed_spacer(20.px())
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_props(Properties::new().with(LineBreaking::Clip)),
                )
                .width(180.px())
                .with_auto_id(),
            )
            .with_fixed_spacer(20.px())
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_props(Properties::new().with(LineBreaking::Overflow)),
                )
                .width(180.px())
                .with_auto_id(),
            )
            .with_spacer(1.0)
            .with_auto_id();

        let mut harness =
            TestHarness::create_with_size(test_property_set(), widget, Size::new(200.0, 200.0));

        assert_render_snapshot!(harness, "label_line_break_modes");
    }

    #[test]
    fn edit_label() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_style(FontFamily::Generic(GenericFamily::Monospace))
                .with_style(StyleProperty::FontSize(20.0))
                .with_text_alignment(TextAlign::Center)
                .with_props(
                    Properties::new()
                        .with(ContentColor::new(ACCENT_COLOR))
                        .with(LineBreaking::WordWrap),
                );

            let mut harness =
                TestHarness::create_with_size(test_property_set(), label, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let label = Label::new("Hello world")
                .with_style(StyleProperty::FontSize(40.0))
                .with_auto_id();

            let mut harness =
                TestHarness::create_with_size(test_property_set(), label, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut label| {
                label.insert_prop(ContentColor::new(ACCENT_COLOR));
                label.insert_prop(LineBreaking::WordWrap);
                Label::set_text(&mut label, "The quick brown fox jumps over the lazy dog");
                Label::insert_style(&mut label, FontFamily::Generic(GenericFamily::Monospace));
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
                Label::set_text_alignment(&mut label, TextAlign::Center);
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
