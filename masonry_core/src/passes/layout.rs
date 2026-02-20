// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The layout pass, which provides the size and position of each widget
//! before any translations applied in [`compose`](crate::passes::compose).
//!
//! The framework helps orchestrate the size computation alongside [`Widget::measure`].
//! The final chosen size is passed to [`Widget::layout`].

use dpi::LogicalSize;
use tracing::{info_span, trace};
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootSignal, RenderRootState, WindowSizePolicy};
use crate::core::{
    ChildrenIds, DefaultProperties, LayoutCtx, MeasureCtx, PropertiesRef, Widget, WidgetArenaNode,
    WidgetState,
};
use crate::kurbo::{Axis, Insets, Point, Size};
use crate::layout::{LayoutSize, LenDef, LenReq, MeasurementInputs, SizeDef};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::properties::{BorderWidth, BoxShadow, Dimensions, Padding};
use crate::util::Sanitize;

// --- MARK: COMPUTE SIZE

/// Measures the preferred border-box length of `widget` on the given `axis`.
///
/// The returned length will be in device pixels.
/// Given that it will be the result of measuring,
/// it must be [sanitized] before passing it back to a widget.
///
/// `len_req` must be [sanitized] before being passed to this function.
///
/// `cross_length`, if present, must be [sanitized] and in device pixels.
///
/// [sanitized]: Sanitize
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "Widget::measure takes props by ref"
)]
fn measure_border_box(
    widget: &mut dyn Widget,
    ctx: &mut MeasureCtx<'_>,
    props: &PropertiesRef<'_>,
    axis: Axis,
    len_req: LenReq,
    cross_length: Option<f64>,
) -> f64 {
    // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
    //       https://github.com/linebender/xilem/issues/1264
    let scale = 1.0;

    let border = props.get::<BorderWidth>();
    let padding = props.get::<Padding>();

    let border_length = border.length(axis).dp(scale);
    let padding_length = padding.length(axis).dp(scale);

    // Reduce the border-box length by the border and padding length to get the content-box length.
    let len_req = len_req.reduce(border_length + padding_length);
    let cross_length = cross_length.map(|cross_length| {
        let cross = axis.cross();
        let cross_border_length = border.length(cross).dp(scale);
        let cross_padding_length = padding.length(cross).dp(scale);
        (cross_length - cross_border_length - cross_padding_length).max(0.)
    });

    // Measure the content-box length.
    let content_length = widget.measure(ctx, props, axis, len_req, cross_length);

    // Add border and padding to the content-box length to return the border-box length.
    content_length + border_length + padding_length
}

/// Resolves the [`LenDef`] of the provided `axis`.
///
/// Unless `Fixed`, this will result in a [`measure`] invocation.
///
/// The returned length will be in device pixels.
/// Given that it can be the result of measuring,
/// it must be [sanitized] before passing it back to a widget.
///
/// `len_def` must be [sanitized] before being passed to this function.
///
/// `cross_length`, if present, must be [sanitized] and in device pixels.
///
/// [sanitized]: Sanitize
/// [`measure`]: Widget::measure
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "Widget::measure takes props by ref"
)]
fn resolve_len_def(
    widget: &mut dyn Widget,
    ctx: &mut MeasureCtx<'_>,
    props: &PropertiesRef<'_>,
    axis: Axis,
    len_def: LenDef,
    cross_length: Option<f64>,
) -> f64 {
    let len_req = match len_def {
        LenDef::Fixed(val) => return val,
        LenDef::MinContent => LenReq::MinContent,
        LenDef::MaxContent => LenReq::MaxContent,
        LenDef::FitContent(space) => LenReq::FitContent(space),
    };

    let inputs = MeasurementInputs::new(axis, len_req, cross_length);
    let cached_result = ctx.widget_state.measurement_cache.get(&inputs);

    #[cfg(debug_assertions)]
    let result = {
        // With debug assertions enabled, we will always measure regardless of cache.
        let result = measure_border_box(widget, ctx, props, axis, len_req, cross_length);
        // If the cache did have the result, we verify that it matches.
        if let Some(cached_result) = cached_result {
            if cached_result != result {
                panic!(
                    "Widget '{}' {} measurement for {inputs:?} returned {result} \
                    but cache already had {cached_result}. Widget::measure needs \
                    to always return the same result for the same inputs. If some other data \
                    changed which influenced the results, you need to request layout!",
                    widget.short_type_name(),
                    ctx.widget_id(),
                );
            }
            return cached_result;
        }
        result
    };
    #[cfg(not(debug_assertions))]
    let result = {
        if let Some(cached_result) = cached_result {
            return cached_result;
        }
        measure_border_box(widget, ctx, props, axis, len_req, cross_length)
    };

    if ctx.cache_result {
        ctx.widget_state.measurement_cache.insert(inputs, result);
    }

    result
}

/// Resolves the widget's preferred border-box length on the given `axis`.
///
/// The returned length will be finite, non-negative, and in device pixels.
///
/// `auto_length` specifies the fallback behavior if a widget's dimension is [`Dim::Auto`].
///
/// `context_size` must be in device pixels.
///
/// `cross_length`, if present, must be finite, non-negative, and in device pixels.
/// Invalid `cross_length` value is fall back to `None`.
///
/// # Panics
///
/// Panics if `auto_length` has a non-finite or negative value and debug assertions are enabled.
///
/// Panics if `cross_length` is non-finite or negative and debug assertions are enabled.
///
/// Panics if a dimension resolves to a non-finite or negative value
/// and debug assertions are enabled. This can happen if the involved numbers are huge,
/// e.g. a logical size of `f64::MAX` scaled by `1.5`.
///
/// Panics if [`Widget::measure`] returned a non-finite or negative length
/// and debug assertions are enabled.
///
/// [`Dim::Auto`]: crate::layout::Dim::Auto
pub(crate) fn resolve_length(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    node: ArenaMut<'_, WidgetArenaNode>,
    auto_length: LenDef,
    context_size: LayoutSize,
    axis: Axis,
    cross_length: Option<f64>,
) -> f64 {
    // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
    //       https://github.com/linebender/xilem/issues/1264
    let scale = 1.0;

    // Sanitize inputs early & always, to quickly catch bugs,
    // because not every code path will use these values.
    let auto_length = auto_length.sanitize("auto_length");
    let cross_length = cross_length.sanitize("cross_length");
    // LayoutSize encapsulates sanitization already.

    // Get the dimensions
    let widget = &mut *node.item.widget;
    let props = PropertiesRef {
        set: &mut node.item.properties,
        default_map: default_properties.for_widget(widget.type_id()),
    };
    let dims = props.get::<Dimensions>();

    // Resolve the dimension on the given axis
    let len_def = dims
        .dim(axis)
        .resolve(scale, context_size.length(axis))
        .unwrap_or(auto_length)
        .sanitize("len_def");

    // Return immediately if we already have a fixed length
    if let LenDef::Fixed(length) = len_def {
        return length;
    }

    // Otherwise fall back to measurement
    let mut children = node.children;
    let mut ctx = MeasureCtx {
        global_state,
        widget_state: &mut node.item.state,
        children: children.reborrow_mut(),
        default_properties,
        auto_length,
        context_size,
        cache_result: true,
    };

    // Resolve the cross dimension in case it's fixed
    let cross_length = cross_length.or_else(|| {
        let cross = axis.cross();
        dims.dim(cross)
            .resolve(scale, context_size.length(cross))
            .and_then(|cross_len_def| cross_len_def.sanitize("cross_len_def").fixed())
    });

    // Measure
    let length = resolve_len_def(widget, &mut ctx, &props, axis, len_def, cross_length);
    length.sanitize("measured length")
}

/// Resolves the widget's preferred border-box size.
///
/// The returned size will be finite, non-negative, and in device pixels.
///
/// `size_def` specifies the fallback behavior if a widget's dimension is [`Dim::Auto`].
///
/// `context_size` must be in device pixels.
///
/// # Panics
///
/// Panics if a dimension resolves to a non-finite or negative value
/// and debug assertions are enabled. This can happen if the involved numbers are huge,
/// e.g. a logical size of `f64::MAX` scaled by `1.5`.
///
/// Panics if [`Widget::measure`] returned a non-finite or negative length
/// and debug assertions are enabled.
///
/// [`Dim::Auto`]: crate::layout::Dim::Auto
pub(crate) fn resolve_size(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    node: ArenaMut<'_, WidgetArenaNode>,
    auto_size: SizeDef,
    context_size: LayoutSize,
) -> Size {
    // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
    //       https://github.com/linebender/xilem/issues/1264
    let scale = 1.0;

    // Input sanitization is not required, because SizeDef and LayoutSize encapsulate it.

    // Currently we only support the common horizontal-tb writing mode,
    // so the assignments are hardcoded here, but the rest of the function adapts.
    let (inline, block) = (Axis::Horizontal, Axis::Vertical);

    // Get the dimensions
    let widget = &mut *node.item.widget;
    let props = PropertiesRef {
        set: &mut node.item.properties,
        default_map: default_properties.for_widget(widget.type_id()),
    };
    let dims = props.get::<Dimensions>();

    // Resolve the dimensions
    let inline_auto = auto_size.dim(inline);
    let inline_def = dims
        .dim(inline)
        .resolve(scale, context_size.length(inline))
        .unwrap_or(inline_auto)
        .sanitize("inline_def");
    let block_auto = auto_size.dim(block);
    let block_def = dims
        .dim(block)
        .resolve(scale, context_size.length(block))
        .unwrap_or(block_auto)
        .sanitize("block_def");

    // Return immediately if we already have a fixed size
    let inline_length = inline_def.fixed();
    let block_length = block_def.fixed();
    if let Some(inline_length) = inline_length
        && let Some(block_length) = block_length
    {
        return inline.pack_size(inline_length, block_length);
    }

    // Otherwise fall back to measurement
    let mut children = node.children;
    let mut ctx = MeasureCtx {
        global_state,
        widget_state: &mut node.item.state,
        children: children.reborrow_mut(),
        default_properties,
        auto_length: inline_auto,
        context_size,
        cache_result: true,
    };

    let inline_length = inline_length.unwrap_or_else(|| {
        resolve_len_def(widget, &mut ctx, &props, inline, inline_def, block_length)
            .sanitize("measured inline length")
    });

    // Update the auto length
    ctx.auto_length = block_auto;

    // Even if the inline measurement couldn't cache, the block one might be able to.
    ctx.cache_result = true;

    let block_length = block_length.unwrap_or_else(|| {
        resolve_len_def(
            widget,
            &mut ctx,
            &props,
            block,
            block_def,
            Some(inline_length),
        )
        .sanitize("measured block length")
    });

    inline.pack_size(inline_length, block_length)
}

// --- MARK: RUN LAYOUT
/// Run [`Widget::layout`] method on the given widget.
/// This will be called by [`LayoutCtx::run_layout`], which is itself called in the parent widget's `layout`.
///
/// The provided `size` will be the given widget's chosen border-box size.
///
/// If the chosen border-box `size` is smaller than what is required to fit the widget's
/// borders and padding, then the `size` will be expanded to meet those constraints.
///
/// The provided `size` must be finite, non-negative, and in device pixels.
/// Non-finite or negative length will fall back to zero with a logged warning.
///
/// # Panics
///
/// Panics if `size` is non-finite or negative and debug assertions are enabled.
///
/// [`Widget::layout`]: crate::core::Widget::layout
pub(crate) fn run_layout_on(
    global_state: &mut RenderRootState,
    default_properties: &DefaultProperties,
    node: ArenaMut<'_, WidgetArenaNode>,
    chosen_size: Size,
) {
    // Ensure the chosen size is sanitized.
    let chosen_size = Size::new(
        chosen_size.width.sanitize("chosen border-box size width"),
        chosen_size.height.sanitize("chosen border-box size height"),
    );

    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let id = state.id;
    let trace = global_state.trace.layout;
    let _span = enter_span_if(trace, state);

    // This checks reads `is_explicitly_stashed` instead of `is_stashed` because the latter may be outdated.
    // A widget's `is_explicitly_stashed` flag is controlled by its direct parent.
    // The parent may set this flag during layout, in which case it should avoid calling `run_layout`.
    // Note that, because this check exits before recursing, `run_layout` can only ever be
    // reached for a widget whose parent is not stashed, which means `is_explicitly_stashed`
    // being false is sufficient to know the widget is non-stashed.
    if state.is_explicitly_stashed {
        debug_panic!(
            "Error in '{}' {}: trying to compute layout of stashed widget.",
            widget.short_type_name(),
            id,
        );
        state.origin = Point::ZERO;
        state.end_point = Point::ZERO;
        state.layout_border_box_size = Size::ZERO;
        return;
    }

    // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
    //       https://github.com/linebender/xilem/issues/1264
    let scale = 1.0;

    let props = PropertiesRef {
        set: properties,
        default_map: default_properties.for_widget(widget.type_id()),
    };

    let border_width = props.get::<BorderWidth>();
    let padding = props.get::<Padding>();

    // Force the border-box size to be large enough to actually contain the border and padding.
    let minimum_size = Size::ZERO;
    let minimum_size = border_width.size_up(minimum_size, scale);
    let minimum_size = padding.size_up(minimum_size, scale);
    let border_box_size = minimum_size.max(chosen_size);

    if !state.needs_layout() && state.layout_border_box_size == border_box_size {
        // We reset this to false to mark that the current widget has been visited.
        state.request_layout = false;
        return;
    }
    state.layout_border_box_size = border_box_size;

    // TODO - Not everything that has been re-laid out needs to be repainted.
    state.needs_paint = true;
    state.needs_compose = true;
    state.needs_accessibility = true;
    state.request_pre_paint = true;
    state.request_paint = true;
    state.request_post_paint = true;
    state.request_compose = true;
    state.request_accessibility = true;

    if trace {
        trace!(
            "Computing layout with border-box size {:?}",
            border_box_size
        );
    }

    // Again, these two blocks read `is_explicitly_stashed` instead of `is_stashed`
    // because the latter may be outdated if layout code has called `set_stashed`.

    let mut children_ids = ChildrenIds::new();
    if cfg!(debug_assertions) {
        children_ids = widget.children_ids();

        // We forcefully set request_layout to true for all children.
        // This is used below to check that widget.layout(..) visited all of them.
        for child_id in widget.children_ids() {
            let child_state = &mut children.item_mut(child_id).unwrap().item.state;
            if !child_state.is_explicitly_stashed {
                child_state.request_layout = true;
            }
        }
    }

    // If children are stashed, the layout pass will not recurse over them.
    // We reset need_layout and request_layout to false directly instead.
    recurse_on_children(id, widget, children.reborrow_mut(), |node| {
        if node.item.state.is_explicitly_stashed {
            clear_layout_flags(node);
        }
    });

    state.paint_insets = Insets::ZERO;

    // Compute the insets for deriving the content-box from the border-box
    let border_box_insets = border_width.insets_up(Insets::ZERO, scale);
    let border_box_insets = padding.insets_up(border_box_insets, scale);
    state.border_box_insets = border_box_insets;

    // Compute the content-box size
    let content_box_size = border_width.size_down(border_box_size, scale);
    let content_box_size = padding.size_down(content_box_size, scale);

    let mut ctx = LayoutCtx {
        global_state,
        widget_state: state,
        children: children.reborrow_mut(),
        default_properties,
    };

    // Run the widget's layout
    widget.layout(&mut ctx, &props, content_box_size);

    // Make sure the paint insets cover the shadow insets
    let shadow = props.get::<BoxShadow>();
    if shadow.is_visible() {
        let shadow_insets = shadow.get_insets();
        state.paint_insets = Insets {
            x0: state.paint_insets.x0.max(shadow_insets.x0),
            y0: state.paint_insets.y0.max(shadow_insets.y0),
            x1: state.paint_insets.x1.max(shadow_insets.x1),
            y1: state.paint_insets.y1.max(shadow_insets.y1),
        };
    }

    if trace {
        trace!(
            "Computed layout: border-box={}, baseline={}, insets={:?}",
            border_box_size, state.layout_baseline_offset, state.paint_insets,
        );
    }

    state.request_layout = false;
    state.set_needs_layout(false);
    state.is_expecting_place_child_call = true;

    #[cfg(debug_assertions)]
    {
        let name = widget.short_type_name();
        for child_id in widget.children_ids() {
            let child_state = &children.item(child_id).unwrap().item.state;

            if child_state.is_explicitly_stashed {
                continue;
            }

            if child_state.request_layout {
                debug_panic!(
                    "Error in '{}' {}: LayoutCtx::run_layout() was not called with child widget '{}' {}.",
                    name,
                    id,
                    child_state.widget_name,
                    child_state.id,
                );
            }

            if child_state.is_expecting_place_child_call {
                debug_panic!(
                    "Error in '{}' {}: LayoutCtx::place_child() was not called with child widget '{}' {}.",
                    name,
                    id,
                    child_state.widget_name,
                    child_state.id,
                );
            }
        }

        let new_children_ids = widget.children_ids();
        if children_ids != new_children_ids && !state.children_changed {
            debug_panic!(
                "Error in '{}' {}: children changed during layout pass",
                name,
                id,
            );
        }
    }
}

// --- MARK: CLEAR LAYOUT
// This function is called on stashed widgets and their children
// to set all layout flags to false.
fn clear_layout_flags(node: ArenaMut<'_, WidgetArenaNode>) {
    let children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let id = state.id;

    state.set_needs_layout(false);
    state.request_layout = false;

    recurse_on_children(id, widget, children, |node| {
        clear_layout_flags(node);
    });
}

// --- MARK: PLACE WIDGET
/// Places the child at `origin` in its parent's border-box coordinate space.
pub(crate) fn place_widget(child_state: &mut WidgetState, origin: Point) {
    let end_point = origin + child_state.layout_border_box_size.to_vec2();
    let baseline_y = end_point.y - child_state.layout_baseline_offset;
    // TODO - Account for display scale in pixel snapping
    // See https://github.com/linebender/xilem/issues/1264
    let origin = origin.round();
    let end_point = end_point.round();
    let baseline_y = baseline_y.round();

    // TODO - We may want to invalidate in other cases as well
    if origin != child_state.origin {
        child_state.transform_changed = true;
    }
    child_state.origin = origin;
    child_state.end_point = end_point;
    child_state.baseline_y = baseline_y;

    child_state.is_expecting_place_child_call = false;
}

// --- MARK: ROOT
/// See the [passes documentation](crate::doc::pass_system#layout-pass).
pub(crate) fn run_layout_pass(root: &mut RenderRoot) {
    if !root.root_state().needs_layout() {
        return;
    }

    let _span = info_span!("layout").entered();
    root.global_state.needs_pointer_pass = true;

    let window_size = root.get_kurbo_size();
    let mut root_node = root.widget_arena.get_node_mut(root.root_id());
    let root_node_size = match root.size_policy {
        WindowSizePolicy::User => resolve_size(
            &mut root.global_state,
            &root.default_properties,
            root_node.reborrow_mut(),
            SizeDef::fixed(window_size),
            window_size.into(),
        ),
        WindowSizePolicy::Content => resolve_size(
            &mut root.global_state,
            &root.default_properties,
            root_node.reborrow_mut(),
            SizeDef::MAX,
            LayoutSize::NONE,
        ),
    };

    run_layout_on(
        &mut root.global_state,
        &root.default_properties,
        root_node.reborrow_mut(),
        root_node_size,
    );
    place_widget(&mut root_node.item.state, Point::ORIGIN);

    if let WindowSizePolicy::Content = root.size_policy {
        // We use the aligned border-box size, which means that transforms won't affect window size.
        let size = root_node.item.state.border_box_size();
        // TODO: Remove HACK: Until scale factor rework happens, we still need to scale here.
        //       https://github.com/linebender/xilem/issues/1264
        let new_size =
            LogicalSize::new(size.width, size.height).to_physical(root.global_state.scale_factor);
        if root.size != new_size {
            root.size = new_size;
            root.global_state
                .emit_signal(RenderRootSignal::SetSize(new_size));
        }
    }
}
