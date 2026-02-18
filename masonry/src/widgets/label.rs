// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;
use std::mem::Discriminant;

use accesskit::{Node, Role};
use include_doc_path::include_doc_path;
use parley::{FontContext, Layout, LayoutAccessibility, LayoutContext};
use smallvec::SmallVec;
use tracing::{Span, trace_span};
use vello::Scene;

use crate::core::{
    AccessCtx, ArcStr, BrushIndex, ChildrenIds, HasProperty, LayoutCtx, MeasureCtx, NoAction,
    PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, StyleSet, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, render_text,
};
use crate::kurbo::{Affine, Axis, Point, Size};
use crate::layout::LenReq;
use crate::properties::{ContentColor, DisabledContentColor, LineBreaking};
use crate::theme::default_text_styles;
use crate::util::debug_panic;
use crate::{TextAlign, TextAlignOptions, theme};

/// A widget displaying non-interactive text.
///
/// This is useful for creating interactive widgets which internally
/// need support for displaying text, such as a button.
///
/// You can customize the look of this label with the
/// [`LineBreaking`], [`ContentColor`] and [`DisabledContentColor`] properties.
///
#[doc = concat!(
    "![Styled label](",
    include_doc_path!("screenshots/label_styled_label.png"),
    ")",
)]
pub struct Label {
    /// Cached layouts.
    layouts: Vec<TextLayout>,
    /// Time tracking for cache usage.
    cache_time: u8,
    /// The currently active layout index.
    ///
    /// `usize::MAX` works well for none, as it will be overwritten before its used
    /// for layout access, but will be read as-is during cache eviction.
    /// During which any value larger than the cache capacity will be ignored.
    active_layout: usize,

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

/// Text layout computation inputs and output.
struct TextLayout {
    /// Computed text layout.
    layout: Layout<BrushIndex>,
    /// Max advance value that was used when calculating this layout.
    max_advance: Option<f32>,
    /// Text alignment of this layout.
    alignment: TextAlign,
    /// Text alignment width of this layout.
    alignment_width: f32,
    /// Last use timestamp for cache eviction purposes.
    last_used: u8,
}

impl TextLayout {
    /// Text layout width differences less than 0.01 pixels can be considered equal.
    const EPSILON: f32 = 0.01;

    /// Creates a new [`TextLayout`] with the specified `max_advance` constraint and `timestamp`.
    fn new(max_advance: Option<f32>, timestamp: u8) -> Self {
        Self {
            layout: Layout::new(),
            max_advance,
            alignment: TextAlign::Start,
            alignment_width: -1., // Not aligned yet
            last_used: timestamp,
        }
    }

    /// Discards this layout, making it an obvious choice for cache eviction.
    fn discard(&mut self) {
        // Mark it as least recently used.
        self.last_used = 0;
        // Set a fake unlikely-to-be-seen max advance so it won't get any use.
        self.max_advance = Some(-1.);
    }

    /// Align the text layout.
    ///
    /// This method ensures that alignment only happens when the inputs have changed.
    fn align(&mut self, alignment: TextAlign, alignment_width: f32) {
        if self.alignment == alignment
            && (self.alignment_width - alignment_width).abs() < Self::EPSILON
        {
            return;
        }
        self.alignment = alignment;
        self.alignment_width = alignment_width;
        self.layout.align(
            Some(self.alignment_width),
            self.alignment,
            TextAlignOptions::default(),
        );
    }

    /// Returns `true` if this layout would be the result for `max_advance`.
    ///
    /// For example:
    ///
    /// `compute_layout(max_advance == 10) => layout.width == 8`
    /// is also valid for `max_advance == 9`.
    ///
    /// This check assumes that Parley does greedy line-breaking,
    /// which it does at the time of writing this.
    fn satisfies(&self, max_advance: Option<f32>) -> bool {
        // Check if the specified constraint is compatible with this layout's constraint.
        self.max_advance.is_none_or(|layout_max_advance| {
            max_advance.is_some_and(|max_advance| {
                layout_max_advance - max_advance + Self::EPSILON >= 0.
            })
        }) &&
        // Check if the computed layout fits into the specified constraint.
        max_advance.is_none_or(|max_advance| {
            max_advance - self.layout.width() + Self::EPSILON >= 0.
        })
    }

    /// Returns `true` if this layout was more constrained than `max_advance`.
    fn more_constrained(&self, max_advance: Option<f32>) -> bool {
        self.max_advance.is_some_and(|layout_max_advance| {
            max_advance.is_none_or(|max_advance| layout_max_advance < max_advance)
        })
    }

    /// Returns `true` if the layouts are equal.
    ///
    /// That is if they have the same number of line breaks with the same reason at the same places.
    fn equals(&self, other: &Self) -> bool {
        if self.layout.len() != other.layout.len() {
            return false;
        }
        let mut a = self.layout.lines();
        let mut b = other.layout.lines();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(a_line), Some(b_line)) => {
                    if a_line.break_reason() != b_line.break_reason() {
                        return false;
                    }
                    if a_line.text_range() != b_line.text_range() {
                        return false;
                    }
                }
                _ => return false,
            }
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
            layouts: Vec::new(),
            cache_time: 0,
            active_layout: usize::MAX,
            text: text.into(),
            styles,
            text_alignment: TextAlign::Start,
            hint: true,
            accessibility: LayoutAccessibility::default(),
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

        this.widget.clear_cache();
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
    /// Of note, behaviour is unspecified for unsetting the [`FontSize`](parley::StyleProperty::FontSize).
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
        self.active_layout = usize::MAX;
    }

    /// Total number of text layouts to cache.
    ///
    /// Must be at least `2`, to allow for one active layout and one speculative one.
    /// Must be less than `u8::MAX` because it's also used as the cache time reset value.
    const CACHE_CAPACITY: usize = 5;

    /// Increments and returns the cache timestamp.
    fn cache_time(&mut self) -> u8 {
        if self.cache_time == u8::MAX {
            // Compress all last_used timestamps
            let n = self.layouts.len();
            let mut idx: SmallVec<[usize; Self::CACHE_CAPACITY]> = (0..n).collect();
            idx.sort_unstable_by_key(|&i| self.layouts[i].last_used);
            for (rank, &i) in idx.iter().enumerate() {
                self.layouts[i].last_used = rank as u8;
            }
            self.cache_time = Self::CACHE_CAPACITY as u8;
        } else {
            self.cache_time += 1;
        }
        self.cache_time
    }

    /// Builds the text layout and breaks the text into lines.
    ///
    /// Backed by a cache layer.
    fn build_and_break(
        &mut self,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<BrushIndex>,
        max_advance: Option<f32>,
    ) -> usize {
        let timestamp = self.cache_time();

        // Check if the cache already has a suitable entry.
        // A suitable entry is one that was calculated with the same or larger constraint,
        // and resulted in a layout that still fits within this newly requested constraint.
        for (idx, layout) in self.layouts.iter_mut().enumerate() {
            if layout.satisfies(max_advance) {
                layout.last_used = timestamp;
                return idx;
            }
        }

        // No known compatible cache entry, so need to do text layout.
        let (mut idx, layout) = if self.layouts.len() < Self::CACHE_CAPACITY {
            // Create a new cache entry.
            self.layouts.push(TextLayout::new(max_advance, timestamp));
            (self.layouts.len() - 1, self.layouts.last_mut().unwrap())
        } else {
            // Repurpose the least recently used non-active cache entry.
            let (idx, layout) = self
                .layouts
                .iter_mut()
                .enumerate()
                .filter(|(idx, _)| *idx != self.active_layout)
                .min_by(|a, b| a.1.last_used.cmp(&b.1.last_used))
                .unwrap();
            layout.max_advance = max_advance;
            layout.last_used = timestamp;
            (idx, layout)
        };

        // TODO: Should we use a different scale?
        // See https://github.com/linebender/xilem/issues/1264
        let mut builder = layout_ctx.ranged_builder(font_ctx, &self.text, 1.0, true);
        for prop in self.styles.inner().values() {
            builder.push_default(prop.to_owned());
        }
        builder.build_into(&mut layout.layout, &self.text);

        layout.layout.break_all_lines(max_advance);

        // Check if the layout result matches an existing cache entry.
        // This happens when slightly increasing max_advance, as we can't then safely pre-identify
        // an existing cache entry because more text might fit inside this new larger constraint.
        // However, if it actually resulted in the same layout as with a slightly lower constraint,
        // then we don't want to have two cache entries with the same identical layout result.
        if let Some((equal_idx, _)) = self
            .layouts
            .iter()
            .enumerate()
            // Only those that are more constrained than the new constraint are viable.
            .filter(|(_, layout)| layout.more_constrained(max_advance))
            // We want the one that is closest to the new constraint.
            .max_by(|a, b| {
                // Because we only look at more constrained entries,
                // these are all guaranteed to be Option::Some.
                a.1.max_advance
                    .unwrap()
                    .total_cmp(&b.1.max_advance.unwrap())
            })
            // Make sure that it actually matches the new layout.
            .filter(|(_, layout)| layout.equals(&self.layouts[idx]))
        {
            // Though these two layouts are equal, we want to keep the older one.
            // Because the old one might be the currently active layout.
            let equal_layout = &mut self.layouts[equal_idx];
            // Mark the old layout as applicable up to this new constraint.
            equal_layout.max_advance = max_advance;
            equal_layout.last_used = timestamp;

            // Discard the new layout that we just created as it is a duplicate.
            let layout = &mut self.layouts[idx];
            layout.discard();

            // Return the updated old entry.
            idx = equal_idx;
        }

        idx
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
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.clear_cache();
                ctx.request_layout();
            }
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
        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        let line_break_mode = props.get::<LineBreaking>();

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
            LineBreaking::Clip | LineBreaking::Overflow => None,
        }
        .map(|v| v as f32);

        let (font_ctx, layout_ctx) = ctx.text_contexts();
        let layout_idx = self.build_and_break(font_ctx, layout_ctx, max_advance);
        let layout = &self.layouts[layout_idx];

        let length = if axis == inline {
            layout.layout.width() // Inline length
        } else {
            layout.layout.height() // Block length
        };

        length as f64
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // Currently we only support the common horizontal-tb writing mode,
        // so we hardcode the assumption that inline axis is horizontal.
        let inline = Axis::Horizontal;

        let line_break_mode = props.get::<LineBreaking>();

        let inline_space = size.get_coord(inline) as f32;

        let max_advance = match line_break_mode {
            LineBreaking::WordWrap => Some(inline_space),
            LineBreaking::Clip | LineBreaking::Overflow => None,
        };

        let (font_ctx, layout_ctx) = ctx.text_contexts();
        self.active_layout = self.build_and_break(font_ctx, layout_ctx, max_advance);
        let layout = &mut self.layouts[self.active_layout];

        layout.align(self.text_alignment, inline_space);

        let baseline = 0.; // TODO: Use actual baseline, at least for single line text
        ctx.set_baseline_offset(baseline);

        if *line_break_mode == LineBreaking::Clip {
            let border_box = size.to_rect() + ctx.border_box_insets();
            ctx.set_clip_path(border_box);
        } else {
            ctx.clear_clip_path();
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let text_color = if ctx.is_disabled()
            && let Some(dc) = props.get_defined::<DisabledContentColor>()
        {
            &dc.0
        } else {
            props.get::<ContentColor>()
        };

        let layout = &self.layouts[self.active_layout];

        render_text(
            scene,
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
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let text_origin_in_border_box_space = Point::ORIGIN + ctx.border_box_translation();

        let layout = &self.layouts[self.active_layout];

        self.accessibility.build_nodes(
            self.text.as_ref(),
            &layout.layout,
            ctx.tree_update(),
            node,
            AccessCtx::next_node_id,
            text_origin_in_border_box_space.x,
            text_origin_in_border_box_space.y,
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
    use crate::core::{NewWidget, PropertySet};
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
                PropertySet::new()
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
            .with_props(PropertySet::new().with(LineBreaking::WordWrap));

        let window_size = Size::new(100.0, 40.0);
        let mut harness = TestHarness::create_with_size(test_property_set(), label, window_size);

        assert_render_snapshot!(harness, "label_underline_label");
    }
    #[test]
    fn strikethrough_label() {
        let label = Label::new("Tpyo")
            .with_style(StyleProperty::Strikethrough(true))
            .with_style(StyleProperty::StrikethroughSize(Some(4.)))
            .with_props(PropertySet::new().with(LineBreaking::WordWrap));

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
                        .with_props(PropertySet::new().with(LineBreaking::WordWrap)),
                )
                .width(180.px())
                .with_auto_id(),
            )
            .with_fixed_spacer(20.px())
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_props(PropertySet::new().with(LineBreaking::Clip)),
                )
                .width(180.px())
                .with_auto_id(),
            )
            .with_fixed_spacer(20.px())
            .with_fixed(
                SizedBox::new(
                    Label::new("The quick brown fox jumps over the lazy dog")
                        .with_props(PropertySet::new().with(LineBreaking::Overflow)),
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
                    PropertySet::new()
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
