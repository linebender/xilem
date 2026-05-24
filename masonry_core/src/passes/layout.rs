// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The layout pass, which provides the size and position of each widget
//! before any translations applied in [`compose`](crate::passes::compose).
//!
//! The framework helps orchestrate the size computation alongside [`Widget::measure`].
//! The layout size is passed to [`Widget::layout`].

use dpi::LogicalSize;
use tracing::{info_span, trace};
use tree_arena::ArenaMut;

use crate::app::{RenderRoot, RenderRootSignal, RenderRootState, WindowSizePolicy};
use crate::core::{
    ChildrenIds, LayoutCtx, MeasureCtx, PropertiesRef, PropertyArena, Widget, WidgetArenaNode,
    WidgetState,
};
use crate::kurbo::{Affine, Axis, Insets, Point, Size};
use crate::layout::{
    LayoutSize, LenDef, LenReq, Length, MeasurementInputs, SizeDef, SnapKey, snap_border_box,
    snap_translation_delta, supports_box_snapping,
};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::properties::{BorderWidth, BoxShadow, Dimensions, Padding};
use crate::util::Sanitize;

// --- MARK: COMPUTE SIZE

/// Measures the preferred border-box length of `widget` on the given `axis`.
fn measure_border_box(
    widget: &mut dyn Widget,
    ctx: &mut MeasureCtx<'_>,
    props: &PropertiesRef<'_>,
    axis: Axis,
    len_req: LenReq,
    cross_length: Option<Length>,
) -> Length {
    let cache = ctx.property_cache();
    let border = props.get::<BorderWidth>(cache);
    let padding = props.get::<Padding>(cache);

    let border_and_padding_length = border.length(axis).saturating_add(padding.length(axis));

    // Reduce the border-box length by the border and padding length to get the content-box length.
    let len_req = len_req.reduce(border_and_padding_length);
    let cross_length = cross_length.map(|cross_length| {
        let cross = axis.cross();
        cross_length
            .saturating_sub(border.length(cross))
            .saturating_sub(padding.length(cross))
    });

    // Measure the content-box length.
    let content_length = widget.measure(ctx, props, axis, len_req, cross_length);

    // Add border and padding to the content-box length to return the border-box length.
    content_length.saturating_add(border_and_padding_length)
}

/// Resolves the [`LenDef`] of the provided `axis`.
///
/// Unless `Fixed`, this will result in a [`measure`] invocation.
///
/// [`measure`]: Widget::measure
fn resolve_len_def(
    widget: &mut dyn Widget,
    ctx: &mut MeasureCtx<'_>,
    props: &PropertiesRef<'_>,
    axis: Axis,
    len_def: LenDef,
    cross_length: Option<Length>,
) -> Length {
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
/// `auto_length` specifies the fallback behavior if a widget's dimension is [`Dim::Auto`].
///
/// # Panics
///
/// Panics if a dimension resolves to a non-finite or negative value
/// and debug assertions are enabled. This can happen if the involved numbers are huge,
/// e.g. a logical size of `f64::MAX` scaled by `1.5`.
///
/// [`Dim::Auto`]: crate::layout::Dim::Auto
pub(crate) fn resolve_length(
    global_state: &mut RenderRootState,
    property_arena: &PropertyArena,
    node: ArenaMut<'_, WidgetArenaNode>,
    auto_length: LenDef,
    context_size: LayoutSize,
    axis: Axis,
    cross_length: Option<Length>,
) -> Length {
    // Get the dimensions
    let class_set = &node.item.class_set;
    let cache = &mut node.item.state.property_cache;
    let widget = &mut *node.item.widget;
    let stack = property_arena.get(node.item.state.property_stack_id, widget.type_id());
    let props = PropertiesRef {
        local: &node.item.properties,
        default_map: property_arena
            .default_properties
            .for_widget(widget.type_id()),
        stack,
        class_set,
    };
    let dims = props.get::<Dimensions>(cache);

    // Resolve the dimension on the given axis
    let len_def = dims
        .dim(axis)
        .resolve(context_size.length(axis))
        .unwrap_or(auto_length);

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
        property_arena,
        auto_length,
        context_size,
        cache_result: true,
    };

    // Resolve the cross dimension in case it's fixed
    let cross_length = cross_length.or_else(|| {
        let cross = axis.cross();
        dims.dim(cross)
            .resolve(context_size.length(cross))
            .and_then(|cross_len_def| cross_len_def.fixed())
    });

    // Measure
    resolve_len_def(widget, &mut ctx, &props, axis, len_def, cross_length)
}

/// Resolves the widget's preferred border-box size.
///
/// The returned size will be finite, non-negative, and in logical pixels.
///
/// `auto_size` specifies the fallback behavior if a widget's dimension is [`Dim::Auto`].
///
/// # Panics
///
/// Panics if a dimension resolves to a non-finite or negative value
/// and debug assertions are enabled. This can happen if the involved numbers are huge,
/// e.g. a logical size of `f64::MAX` scaled by `1.5`.
///
/// [`Dim::Auto`]: crate::layout::Dim::Auto
pub(crate) fn resolve_size(
    global_state: &mut RenderRootState,
    property_arena: &PropertyArena,
    node: ArenaMut<'_, WidgetArenaNode>,
    auto_size: SizeDef,
    context_size: LayoutSize,
) -> Size {
    // Currently we only support the common horizontal-tb writing mode,
    // so the assignments are hardcoded here, but the rest of the function adapts.
    let (inline, block) = (Axis::Horizontal, Axis::Vertical);

    // Get the dimensions
    let class_set = &node.item.class_set;
    let cache = &mut node.item.state.property_cache;
    let widget = &mut *node.item.widget;
    let stack = property_arena.get(node.item.state.property_stack_id, widget.type_id());
    let props = PropertiesRef {
        local: &node.item.properties,
        default_map: property_arena
            .default_properties
            .for_widget(widget.type_id()),
        stack,
        class_set,
    };
    let dims = props.get::<Dimensions>(cache);

    // Resolve the dimensions
    let inline_auto = auto_size.dim(inline);
    let inline_def = dims
        .dim(inline)
        .resolve(context_size.length(inline))
        .unwrap_or(inline_auto);
    let block_auto = auto_size.dim(block);
    let block_def = dims
        .dim(block)
        .resolve(context_size.length(block))
        .unwrap_or(block_auto);

    // Return immediately if we already have a fixed size
    let inline_length = inline_def.fixed();
    let block_length = block_def.fixed();
    if let Some(inline_length) = inline_length
        && let Some(block_length) = block_length
    {
        return inline.pack_size(inline_length.get(), block_length.get());
    }

    // Otherwise fall back to measurement
    let mut children = node.children;
    let mut ctx = MeasureCtx {
        global_state,
        widget_state: &mut node.item.state,
        children: children.reborrow_mut(),
        property_arena,
        auto_length: inline_auto,
        context_size,
        cache_result: true,
    };

    let inline_length = inline_length.unwrap_or_else(|| {
        resolve_len_def(widget, &mut ctx, &props, inline, inline_def, block_length)
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
    });

    inline.pack_size(inline_length.get(), block_length.get())
}

// --- MARK: RUN LAYOUT
/// Places the widget based on `chosen_origin` and runs its [`Widget::layout`] method.
///
/// This will be called by [`LayoutCtx::layout_child`], which is itself called
/// in the parent widget's `layout`.
///
/// The provided `chosen_size` will be the given widget's chosen border-box size,
/// before minimum border/padding constraints and pixel snapping are applied.
///
/// The provided `chosen_size` must be finite, non-negative, and in logical pixels.
/// Non-finite or negative length will fall back to zero with a logged warning.
///
/// The provided `chosen_origin` must be finite and in logical pixels.
/// Non-finite origin will fall back to zero with a logged warning.
///
/// # Panics
///
/// Panics if `chosen_size` is non-finite or negative and debug assertions are enabled.
///
/// Panics if `chosen_origin` is non-finite and debug assertions are enabled.
///
/// [`Widget::layout`]: crate::core::Widget::layout
pub(crate) fn run_layout_on(
    global_state: &mut RenderRootState,
    property_arena: &PropertyArena,
    node: ArenaMut<'_, WidgetArenaNode>,
    chosen_origin: Point,
    chosen_size: Size,
    parent_snap_transform: Affine,
    parent_snap_disabled: bool,
    parent_snap_unsupported: bool,
) {
    // Ensure the chosen origin is sanitized.
    let chosen_origin = if chosen_origin.is_finite() {
        chosen_origin
    } else {
        debug_panic!("chosen origin must be finite, got {}", chosen_origin);
        Point::ZERO
    };

    // Ensure the chosen size is sanitized.
    let chosen_size = Size::new(
        chosen_size.width.sanitize("chosen border-box size width"),
        chosen_size.height.sanitize("chosen border-box size height"),
    );

    let mut children = node.children;
    let widget = &mut *node.item.widget;
    let state = &mut node.item.state;
    let properties = &mut node.item.properties;
    let class_set = &node.item.class_set;
    let id = state.id;
    let trace = global_state.trace.layout;
    let _span = enter_span_if(trace, state);

    // This checks reads `is_explicitly_stashed` instead of `is_stashed` because the latter may be outdated.
    // A widget's `is_explicitly_stashed` flag is controlled by its direct parent.
    // The parent may set this flag during layout, in which case it should avoid calling `layout_child`.
    // Note that, because this check exits before recursing, layout can only ever be
    // reached for a widget whose parent is not stashed, which means `is_explicitly_stashed`
    // being false is sufficient to know the widget is non-stashed.
    if state.is_explicitly_stashed {
        debug_panic!(
            "Error in '{}' {}: trying to compute layout of stashed widget.",
            widget.short_type_name(),
            id,
        );
        state.origin = Point::ZERO;
        state.border_box_size = Size::ZERO;
        return;
    }

    let stack = property_arena.get(state.property_stack_id, widget.type_id());
    let props = PropertiesRef {
        local: properties,
        default_map: property_arena
            .default_properties
            .for_widget(widget.type_id()),
        stack,
        class_set,
    };

    let border_width = props.get::<BorderWidth>(&mut state.property_cache);
    let padding = props.get::<Padding>(&mut state.property_cache);

    // Force the chosen border-box to be large enough to actually contain the border and padding.
    let minimum_size = Size::ZERO;
    let minimum_size = border_width.size_up(minimum_size);
    let minimum_size = padding.size_up(minimum_size);
    let chosen_border_box = minimum_size.max(chosen_size).to_rect();

    // Calculate the chosen snap transform based on the chosen origin.
    // Snap transform excludes scroll translation, which will be dealt with during compose.
    let chosen_snap_transform =
        parent_snap_transform * state.transform.then_translate(chosen_origin.to_vec2());

    // Update the flags that determine whether snapping is active.
    let is_snap_disabled = parent_snap_disabled || state.is_explicitly_snap_disabled;
    let snap_disabled_changed = state.is_snap_disabled != is_snap_disabled;
    state.is_snap_disabled = is_snap_disabled;
    state.is_snap_unsupported = parent_snap_unsupported
        || !supports_box_snapping(chosen_snap_transform.then_scale(global_state.scale_factor));

    // Snap the chosen border-box to the pixel grid.
    let snapped_border_box = if state.is_snap_active() {
        snap_border_box(
            chosen_border_box,
            chosen_snap_transform,
            global_state.scale_factor,
        )
    } else {
        chosen_border_box
    };

    // The layout origin will be the chosen origin adjusted so that it will be pixel-snapped.
    let origin_delta =
        state.transform * snapped_border_box.origin() - state.transform * Point::ORIGIN;
    let origin = chosen_origin + origin_delta;
    if state.origin != origin {
        state.origin = origin;
        state.mark_compose_transform_changed();
    }

    // Now that we have the layout origin, we can calculate the corresponding snap transform.
    let snap_transform =
        parent_snap_transform * state.transform.then_translate(state.origin.to_vec2());

    // The border-box size will be exactly the pixel-snapped chosen border-box size.
    let border_box_size = snapped_border_box.size();

    // We can skip redoing layout for this branch, if all the following conditions are true:
    // - No widget in this branch explicitly requested layout.
    // - The border-box size matches the cached layout.
    //   The box size can change due to snapping, constraints, or just a new chosen input.
    // - The snap key matches the cached layout.
    //   Even though the cached layout for this widget may be valid, if the snap key changed,
    //   then deeper descendant widgets may end up with different border-boxes due to snapping.
    // - The snap disabled state remains the same as it was during the cached layout.
    //   Even though the cached layout for this whole branch may be valid,
    //   we still need to propagate the is_snap_disabled flag updates, as those are used
    //   in context methods like set_transform to decide whether layout or only compose is needed.
    let snap_key = state
        .is_snap_active()
        .then(|| SnapKey::new(snap_transform, global_state.scale_factor));
    if !state.needs_layout()
        && state.border_box_size == border_box_size
        && state.snap_key == snap_key
        && !snap_disabled_changed
    {
        // We reset this to false to mark that the current widget has been visited.
        state.request_layout = false;
        return;
    }
    state.border_box_size = border_box_size;
    state.snap_key = snap_key;

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

    state.paint_box_insets = Insets::ZERO;

    // Compute the insets for deriving the content-box from the border-box
    let border_box_insets = border_width.insets_up(Insets::ZERO);
    let border_box_insets = padding.insets_up(border_box_insets);
    state.border_box_insets = border_box_insets;

    // Compute the content-box size
    let content_box_size = border_width.size_down(border_box_size);
    let content_box_size = padding.size_down(content_box_size);

    let mut ctx = LayoutCtx {
        global_state,
        widget_state: state,
        children: children.reborrow_mut(),
        property_arena,
        snap_transform,
    };

    // Run the widget's layout
    widget.layout(&mut ctx, &props, content_box_size);

    // Make sure the paint insets cover the shadow insets
    let shadow = props.get::<BoxShadow>(&mut state.property_cache);
    if shadow.is_visible() {
        let shadow_insets = shadow.get_insets();
        state.paint_box_insets = Insets {
            x0: state.paint_box_insets.x0.max(shadow_insets.x0),
            y0: state.paint_box_insets.y0.max(shadow_insets.y0),
            x1: state.paint_box_insets.x1.max(shadow_insets.x1),
            y1: state.paint_box_insets.y1.max(shadow_insets.y1),
        };
    }

    if trace {
        trace!(
            "Computed layout: border-box={}, first_baseline={}, last_baseline={} insets={:?}",
            border_box_size, state.first_baseline, state.last_baseline, state.paint_box_insets,
        );
    }

    state.request_layout = false;
    state.set_needs_layout(false);

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
                    "Error in '{}' {}: LayoutCtx::layout_child() was not called with child widget '{}' {}.",
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

// --- MARK: MOVE WIDGET
/// Moves a placed child to `origin`, quantizing the movement to an
/// integer device-pixel delta when snapping is active.
///
/// The provided `chosen_origin` must be finite and in logical pixels.
/// Non-finite origin will fall back to zero with a logged warning.
///
/// # Panics
///
/// Panics if `chosen_origin` is non-finite and debug assertions are enabled.
pub(crate) fn move_widget(
    child_state: &mut WidgetState,
    chosen_origin: Point,
    parent_snap_transform: Affine,
    scale_factor: f64,
) {
    // Ensure the chosen origin is sanitized.
    let chosen_origin = if chosen_origin.is_finite() {
        chosen_origin
    } else {
        debug_panic!("chosen origin must be finite, got {}", chosen_origin);
        Point::ZERO
    };

    // We snap the delta instead of the chosen origin, because then we can skip dealing with
    // the local transform of the child. Which is to say, the current child origin is snapped
    // only after the child's local transform is applied. Adding a snapped delta means that
    // the new origin will also end up snapped after the child's local transform is applied.
    let requested_delta = chosen_origin - child_state.origin;
    let snapped_delta = if child_state.is_snap_active() {
        snap_translation_delta(requested_delta, parent_snap_transform, scale_factor)
    } else {
        requested_delta
    };
    let origin = child_state.origin + snapped_delta;

    if child_state.origin != origin {
        child_state.origin = origin;
        child_state.mark_compose_transform_changed();
    }
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
    let root_node_size = match root.global_state.size_policy {
        WindowSizePolicy::User => resolve_size(
            &mut root.global_state,
            &root.property_arena,
            root_node.reborrow_mut(),
            SizeDef::fixed(window_size),
            window_size.into(),
        ),
        WindowSizePolicy::Content => resolve_size(
            &mut root.global_state,
            &root.property_arena,
            root_node.reborrow_mut(),
            SizeDef::MAX,
            LayoutSize::NONE,
        ),
    };

    run_layout_on(
        &mut root.global_state,
        &root.property_arena,
        root_node.reborrow_mut(),
        Point::ORIGIN,
        root_node_size,
        Affine::IDENTITY,
        false,
        false,
    );

    if let WindowSizePolicy::Content = root.global_state.size_policy {
        // We use the border-box size, which means that transforms won't affect window size.
        let size = root_node.item.state.border_box_size;
        let new_size =
            LogicalSize::new(size.width, size.height).to_physical(root.global_state.scale_factor);
        if root.global_state.size != new_size {
            root.global_state.size = new_size;
            root.global_state
                .emit_signal(RenderRootSignal::SetSize(new_size));
        }
    }
}
