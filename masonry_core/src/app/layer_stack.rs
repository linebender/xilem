// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Axis, Point, Size};

use crate::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::layout::{LenReq, SizeDef};

/// A widget representing the top-level stack of visible layers owned by [`RenderRoot`](crate::app::RenderRoot).
///
/// This stack must always have at least one child, representing the "base layer",
/// e.g. most of the stuff drawn in the app.
///
/// Other layers can represent tooltips, menus, dialogs, etc.
/// They have an associated position and are drawn on top of the base layer.
///
/// Ensure that `LayerStack` has [`Dimensions`] set via props to [`Dimensions::MAX`].
/// Max preferred size of `LayerStack` means that the question of size
/// will get passed through to its base layer, and doesn't mean that it will
/// necessarily map to the max preferred size of the base layer.
///
/// [`Dimensions`]: crate::properties::Dimensions
/// [`Dimensions::MAX`]: crate::properties::Dimensions::MAX
pub(crate) struct LayerStack {
    layers: Vec<Layer>,
}

struct Layer {
    widget: WidgetPod<dyn Widget>,
    pos: Point,
}

// --- MARK: IMPL LAYER_STACK
impl LayerStack {
    /// Creates a stack with the provided base layer.
    pub(crate) fn new(root: NewWidget<impl Widget + ?Sized>) -> Self {
        let layer = Layer {
            widget: root.erased().to_pod(),
            pos: Point::ZERO,
        };
        Self {
            layers: vec![layer],
        }
    }

    /// Returns the number of layers, including the base layer.
    #[expect(dead_code, reason = "Might be useful later")]
    pub(crate) fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// The [`WidgetId`] of the root widget of the given layer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    pub(crate) fn layer_id(&self, idx: usize) -> WidgetId {
        self.layers[idx].widget.id()
    }
}

// --- MARK: IMPL WIDGETMUT
impl LayerStack {
    /// Adds a new layer at the end of the stack, with the given widget as its root, at the given position.
    ///
    /// The given `pos` must be in this `LayerStack`'s content-box coordinate space.
    /// If this `LayerStack` is used as the root widget with no borders, padding, or transforms,
    /// then that coordinate space will exactly match the window's coordinate space.
    pub(crate) fn add_layer(
        this: &mut WidgetMut<'_, Self>,
        root: NewWidget<impl Widget + ?Sized>,
        pos: Point,
    ) {
        let layer = Layer {
            widget: root.erased().to_pod(),
            pos,
        };
        this.widget.layers.push(layer);
        this.ctx.children_changed();
    }

    /// Returns a mutable reference to the root widget of the layer at `idx`.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    #[expect(dead_code, reason = "Might be useful later")]
    pub(crate) fn layer_root_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        idx: usize,
    ) -> WidgetMut<'t, dyn Widget> {
        let layer = &mut this.widget.layers[idx].widget;
        this.ctx.get_mut(layer)
    }

    /// Removes the layer with the given widget as root.
    ///
    /// The base layer cannot be removed.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the the intended layer the base layer or the
    /// intended layer is not found.
    pub(crate) fn remove_layer(this: &mut WidgetMut<'_, Self>, root_id: WidgetId) {
        match this
            .widget
            .layers
            .iter()
            .position(|layer| layer.widget.id() == root_id)
        {
            Some(0) => debug_panic!("Cannot remove initial layer"),
            None => debug_panic!("layer with root widget {root_id:?} not found"),
            Some(idx) => {
                let child = this.widget.layers.remove(idx).widget;
                this.ctx.remove_child(child);
            }
        }
    }

    /// Repositions the layer with the given widget as root.
    ///
    /// The given `new_origin` must be in this `LayerStack`'s content-box coordinate space.
    /// If this `LayerStack` is used as the root widget with no borders, padding, or transforms,
    /// then that coordinate space will exactly match the window's coordinate space.
    ///
    /// The base layer cannot be repositioned.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the the intended layer the base layer or the
    /// intended layer is not found.
    pub(crate) fn reposition_layer(
        this: &mut WidgetMut<'_, Self>,
        root_id: WidgetId,
        new_origin: Point,
    ) {
        match this
            .widget
            .layers
            .iter()
            .position(|layer| layer.widget.id() == root_id)
        {
            Some(0) => debug_panic!("Cannot reposition initial layer"),
            None => debug_panic!("layer with root widget {root_id:?} not found"),
            Some(idx) => {
                this.widget.layers[idx].pos = new_origin;
                this.ctx.request_layout();
            }
        }
    }
}

// --- MARK: IMPL WIDGET
impl Widget for LayerStack {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for layer in self.layers.iter_mut() {
            ctx.register_child(&mut layer.widget);
        }
    }

    fn property_changed(&mut self, _ctx: &mut UpdateCtx<'_>, _property_type: TypeId) {}

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // First child is the base layer, for which we do measure passthrough.
        let Some(base_layer) = self.layers.first_mut() else {
            debug_panic!("Missing first layer");
            return 0.;
        };
        // Let the base layer handle the response
        ctx.redirect_measurement(&mut base_layer.widget, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // First child is the base layer, for which we do layout passthrough.
        let Some(base_layer) = self.layers.first_mut() else {
            debug_panic!("Missing first layer");
            return;
        };

        // Our measurement is just the passed on result from the base layer,
        // so the size we received is effectively meant for the base layer.
        ctx.run_layout(&mut base_layer.widget, size);
        // The base layer is always located at our origin.
        ctx.place_child(&mut base_layer.widget, Point::ORIGIN);

        let baseline = ctx.child_baseline_offset(&base_layer.widget);
        ctx.set_baseline_offset(baseline);

        for layer in &mut self.layers[1..] {
            // Other layers don't take part in our measurement,
            // so RenderRoot has no idea what they want and instead we control them.
            // We don't really care if they go outside the bounds of the base layer,
            // so we won't give any FitContent fallback and instead just use MaxContent.
            // These other layers still have access to the window size via context size.
            let layer_size = ctx.compute_size(&mut layer.widget, SizeDef::MAX, size.into());
            ctx.run_layout(&mut layer.widget, layer_size);
            ctx.place_child(&mut layer.widget, layer.pos);
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        self.layers.iter().map(|child| child.widget.id()).collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("LayerStack", id = id.trace())
    }
}
