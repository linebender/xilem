// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! The context types that are passed into various widget methods.

use std::any::Any;

use accesskit::TreeUpdate;
use anymore::AnyDebug;
use dpi::{LogicalPosition, PhysicalPosition};
use parley::{FontContext, LayoutContext};
use tracing::{trace, warn};
use tree_arena::{ArenaMutList, ArenaRefList};
use vello::kurbo::{Affine, Insets, Point, Rect, Size, Vec2};

use crate::app::{MutateCallback, RenderRootSignal, RenderRootState};
use crate::core::{
    AllowRawMut, BoxConstraints, BrushIndex, DefaultProperties, ErasedAction, FromDynWidget,
    NewWidget, PropertiesMut, PropertiesRef, ResizeDirection, Widget, WidgetArenaNode, WidgetId,
    WidgetMut, WidgetPod, WidgetRef, WidgetState,
};
use crate::debug_panic;
use crate::passes::layout::run_layout_on;
use crate::peniko::Color;
use crate::util::get_debug_color;

// Note - Most methods defined in this file revolve around `WidgetState` fields.
// Consider reading `WidgetState` documentation (especially the documented naming scheme)
// before editing context method code.

/// A macro for implementing methods on multiple contexts.
///
/// There are a lot of methods defined on multiple contexts; this lets us only
/// have to write them out once.
macro_rules! impl_context_method {
    ($ty:ty,  { $($method:item)+ } ) => {
        impl $ty { $($method)+ }
    };
    ( $ty:ty, $($more:ty),+, { $($method:item)+ } ) => {
        impl_context_method!($ty, { $($method)+ });
        impl_context_method!($($more),+, { $($method)+ });
    };
}

/// A context provided inside of [`WidgetMut`].
///
/// When you declare a mutable reference type for your widget, methods of this type
/// will have access to a `MutateCtx`. If that method mutates the widget in a way that
/// requires a later pass (for instance, if your widget has a `set_color` method),
/// you will need to signal that change in the pass (eg [`request_render`](MutateCtx::request_render)).
pub struct MutateCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) parent_widget_state: Option<&'a mut WidgetState>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) properties: PropertiesMut<'a>,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
}

/// A context provided inside of [`WidgetRef`].
///
/// This context is passed to methods of widgets requiring shared, read-only access.
#[derive(Clone, Copy)]
pub struct QueryCtx<'a> {
    pub(crate) global_state: &'a RenderRootState,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) properties: PropertiesRef<'a>,
    pub(crate) children: ArenaRefList<'a, WidgetArenaNode>,
}

/// A context provided to event-handling [`Widget`] methods.
pub struct EventCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) target: WidgetId,
    pub(crate) allow_pointer_capture: bool,
    pub(crate) is_handled: bool,
}

/// A context provided to the [`Widget::register_children`] method.
pub struct RegisterCtx<'a> {
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    #[cfg(debug_assertions)]
    pub(crate) registered_ids: Vec<WidgetId>,
}

/// A context provided to the [`Widget::update`] method.
pub struct UpdateCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
}

// TODO - Change this once other layout methods are added.
/// A context provided to [`Widget::layout`] methods.
pub struct LayoutCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) default_properties: &'a DefaultProperties,
}

/// A context provided to the [`Widget::compose`] method.
pub struct ComposeCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
}

/// A context passed to [`Widget::paint`] method.
pub struct PaintCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) debug_paint: bool,
}

/// A context passed to [`Widget::accessibility`] method.
pub struct AccessCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) tree_update: &'a mut TreeUpdate,
    pub(crate) rebuild_all: bool,
}

// --- MARK: GETTERS
// Methods for all context types
impl_context_method!(
    MutateCtx<'_>,
    QueryCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// The `WidgetId` of the current widget.
        pub fn widget_id(&self) -> WidgetId {
            self.widget_state.id
        }

        #[allow(dead_code, reason = "Copy-pasted for some types that don't need it")]
        /// Helper method to get a direct reference to a child widget from its `WidgetPod`.
        fn get_child<Child: Widget>(&self, child: &'_ WidgetPod<Child>) -> &'_ Child {
            let child_ref = &*self
                .children
                .item(child.id())
                .expect("get_child: child not found")
                .item
                .widget;
            (child_ref as &dyn Any).downcast_ref::<Child>().unwrap()
        }

        #[allow(dead_code, reason = "Copy-pasted for some types that don't need it")]
        /// Helper method to get a direct reference to a child widget from its `WidgetPod`.
        fn get_child_dyn(&self, child: &'_ WidgetPod<impl Widget + ?Sized>) -> &'_ dyn Widget {
            &*self
                .children
                .item(child.id())
                .expect("get_child: child not found")
                .item
                .widget
        }

        #[allow(dead_code, reason = "Copy-pasted for some types that don't need it")]
        /// Helper method to get a direct reference to a child widget's `WidgetState` from its `WidgetPod`.
        fn get_child_state(&self, child: &'_ WidgetPod<impl Widget + ?Sized>) -> &'_ WidgetState {
            &self
                .children
                .item(child.id())
                .expect("get_child_state: child not found")
                .item
                .state
        }

        /// The current (local) transform of this widget.
        pub fn transform(&self) -> Affine {
            self.widget_state.transform
        }
    }
);

impl_context_method!(
    MutateCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    {
        /// Helper method to get a mutable reference to a child widget's `WidgetState` from its `WidgetPod`.
        ///
        /// This one isn't defined for `PaintCtx` and `AccessCtx` because those contexts
        /// can't mutate `WidgetState`.
        #[track_caller]
        fn get_child_state_mut<Child: Widget + ?Sized>(
            &mut self,
            child: &'_ mut WidgetPod<Child>,
        ) -> &'_ mut WidgetState {
            &mut self
                .children
                .item_mut(child.id())
                .expect("get_child_state_mut: child not found")
                .item
                .state
        }
    }
);

// --- MARK: WIDGET_MUT
// Methods to get a child WidgetMut from a parent.
impl MutateCtx<'_> {
    /// Return a [`WidgetMut`] to a child widget.
    pub fn get_mut<'c, Child: Widget + FromDynWidget + ?Sized>(
        &'c mut self,
        child: &'c mut WidgetPod<Child>,
    ) -> WidgetMut<'c, Child> {
        let node_mut = self
            .children
            .item_mut(child.id())
            .expect("get_mut: child not found");
        let child_ctx = MutateCtx {
            global_state: self.global_state,
            parent_widget_state: Some(&mut self.widget_state),
            widget_state: &mut node_mut.item.state,
            properties: PropertiesMut {
                map: &mut node_mut.item.properties,
                default_map: self.properties.default_map,
            },
            children: node_mut.children,
        };
        WidgetMut {
            ctx: child_ctx,
            widget: Child::from_dyn_mut(&mut *node_mut.item.widget).unwrap(),
        }
    }

    pub(crate) fn reborrow_mut(&mut self) -> MutateCtx<'_> {
        MutateCtx {
            global_state: self.global_state,
            // We don't don't reborrow `parent_widget_state`. This avoids running
            // `merge_up` in `WidgetMut::Drop` multiple times for the same state.
            // It will still be called when the original borrow is dropped.
            parent_widget_state: None,
            widget_state: self.widget_state,
            properties: self.properties.reborrow_mut(),
            children: self.children.reborrow_mut(),
        }
    }

    pub(crate) fn update_mut(&mut self) -> UpdateCtx<'_> {
        UpdateCtx {
            global_state: self.global_state,
            widget_state: self.widget_state,
            children: self.children.reborrow_mut(),
        }
    }

    /// Whether the (local) transform of this widget has been modified since
    /// the last time this widget's transformation was resolved.
    ///
    /// This is exposed for Xilem, and is more likely to change or be removed
    /// in major releases of Masonry.
    pub fn transform_has_changed(&self) -> bool {
        self.widget_state.transform_changed
    }
}

// --- MARK: WIDGET_REF
// Methods to get a child WidgetRef from a parent.
impl<'w> QueryCtx<'w> {
    /// Return a [`WidgetRef`] to a child widget.
    pub fn get(self, child: WidgetId) -> WidgetRef<'w, dyn Widget> {
        let child_node = self
            .children
            .into_item(child)
            .expect("get_mut: child not found");
        let child_ctx = QueryCtx {
            global_state: self.global_state,
            widget_state: &child_node.item.state,
            properties: PropertiesRef {
                map: &child_node.item.properties,
                default_map: self.properties.default_map,
            },
            children: child_node.children,
        };
        WidgetRef {
            ctx: child_ctx,
            widget: &*child_node.item.widget,
        }
    }
}

// Methods for all exclusive context types (i.e. those which have exclusive access to the global state).
impl_context_method!(
    MutateCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// Get the Parley contexts needed to build and paint text sections.
        ///
        /// Most users should embed a pre-built label widget
        /// (such as `masonry::widgets::Label`)
        /// as a child for non-interactive text.
        /// These contexts could however be useful for custom text editing, such as for rich text editing.
        pub fn text_contexts(&mut self) -> (&mut FontContext, &mut LayoutContext<BrushIndex>) {
            (
                &mut self.global_state.font_context,
                &mut self.global_state.text_layout_context,
            )
        }
    }
);

// --- MARK: EVENT HANDLING
impl EventCtx<'_> {
    /// Capture the pointer in the current widget.
    ///
    /// [Pointer capture] is only allowed during a [`Down`] event. It is a logic error to
    /// capture the pointer during any other event.
    ///
    /// A widget normally only receives pointer events when the pointer is inside the widget's
    /// layout box. Pointer capture causes widget layout boxes to be ignored: when the pointer is
    /// captured by a widget, that widget will continue receiving pointer events when the pointer
    /// is outside the widget's layout box. Other widgets the pointer is over will not receive
    /// events. Events that are not marked as handled by the capturing widget, bubble up to the
    /// widget's ancestors, ignoring their layout boxes as well.
    ///
    /// The pointer cannot be captured by multiple widgets at the same time. If a widget has
    /// captured the pointer and another widget captures it, the first widget loses the pointer
    /// capture.
    ///
    /// # Releasing the pointer
    ///
    /// Any widget can [`release`] the pointer during any event.
    /// The pointer is automatically released after handling of a [`Up`] or [`Cancel`] event completes.
    /// A widget holding the pointer capture will be the target of these events.
    ///
    /// If pointer capture is lost for external reasons (the widget is disabled, the window lost focus,
    /// etc), the widget will still get a [`Cancel`] event.
    ///
    /// [Pointer capture]: crate::doc::masonry_concepts#pointer-capture
    /// [`Down`]: ui_events::pointer::PointerEvent::Down
    /// [`Up`]: ui_events::pointer::PointerEvent::Up
    /// [`Cancel`]: ui_events::pointer::PointerEvent::Cancel
    /// [`release`]: Self::release_pointer
    #[track_caller]
    pub fn capture_pointer(&mut self) {
        let id = self.widget_id();
        if !self.allow_pointer_capture {
            debug_panic!("capture_pointer - '{id}': event does not allow pointer capture");
            return;
        }
        // TODO: plumb pointer capture through to platform (through winit)
        self.global_state.pointer_capture_target = Some(id);
        self.global_state.needs_pointer_pass = true;
    }

    /// Release the pointer previously [captured] through [`capture_pointer`].
    ///
    /// [captured]: crate::doc::masonry_concepts#pointer-capture
    /// [`capture_pointer`]: EventCtx::capture_pointer
    pub fn release_pointer(&mut self) {
        let id = self.widget_id();
        if self.global_state.pointer_capture_target.is_none() {
            warn!("release_pointer - '{id}': no widget is captured");
            return;
        }
        if self.global_state.pointer_capture_target != Some(self.widget_state.id) {
            warn!("release_pointer - '{id}': widget does not have pointer capture");
            return;
        }
        self.global_state.pointer_capture_target = None;
        self.global_state.needs_pointer_pass = true;
    }

    /// Send a signal to parent widgets to scroll this widget into view.
    pub fn request_scroll_to_this(&mut self) {
        let rect = self.widget_state.layout_rect();
        self.global_state
            .scroll_request_targets
            .push((self.widget_state.id, rect));
    }

    /// Send a signal to parent widgets to scroll this area into view.
    ///
    /// `rect` is in local coordinates.
    pub fn request_scroll_to(&mut self, rect: Rect) {
        self.global_state
            .scroll_request_targets
            .push((self.widget_state.id, rect));
    }

    /// Set the event as "handled", which stops its propagation to parent
    /// widgets.
    pub fn set_handled(&mut self) {
        trace!("set_handled");
        self.is_handled = true;
    }

    /// Determine whether the event has been handled.
    pub fn is_handled(&self) -> bool {
        self.is_handled
    }

    /// The widget originally targeted by the event.
    ///
    /// This will be different from [`widget_id`](Self::widget_id) during event bubbling.
    pub fn target(&self) -> WidgetId {
        self.target
    }

    /// Request [text focus].
    ///
    /// Because only one widget can be focused at a time, multiple focus requests
    /// from different widgets during a single event cycle means that the last
    /// widget that requests focus will override the previous requests.
    ///
    /// [text focus]: crate::doc::masonry_concepts#text-focus
    pub fn request_focus(&mut self) {
        trace!("request_focus");
        // We need to send the request even if we're currently focused,
        // because we may have a sibling widget that already requested focus
        // and we have no way of knowing that yet. We need to override that
        // to deliver on the "last focus request wins" promise.
        let id = self.widget_id();
        self.global_state.next_focused_widget = Some(id);
    }

    /// Transfer [text focus] to the widget with the given `WidgetId`.
    ///
    /// [text focus]: crate::doc::masonry_concepts#text-focus
    pub fn set_focus(&mut self, target: WidgetId) {
        trace!("set_focus target={:?}", target);
        self.global_state.next_focused_widget = Some(target);
    }

    /// Give up [text focus].
    ///
    /// This should only be called by a widget that currently has focus.
    ///
    /// [text focus]: crate::doc::masonry_concepts#text-focus
    pub fn resign_focus(&mut self) {
        trace!("resign_focus");
        if self.has_focus_target() {
            self.global_state.next_focused_widget = None;
        } else {
            warn!(
                "resign_focus can only be called by the currently focused widget {} \
                 or one of its ancestors.",
                self.widget_id()
            );
        }
    }

    /// Translate window position to widget local position.
    pub fn local_position(&self, p: PhysicalPosition<f64>) -> Point {
        let LogicalPosition { x, y } = p.to_logical(self.global_state.scale_factor);
        self.widget_state.window_transform.inverse() * Point { x, y }
    }
}

// --- MARK: ACCESSIBILITY
impl AccessCtx<'_> {
    // TODO - We need access to the TreeUpdate to create sub-nodes for text runs,
    // but this seems too powerful. We should figure out another API.
    /// A mutable reference to the global [`TreeUpdate`] object in which all modified/new
    /// accessibility nodes are stored.
    pub fn tree_update(&mut self) -> &mut TreeUpdate {
        self.tree_update
    }
}

// --- MARK: UPDATE LAYOUT
impl LayoutCtx<'_> {
    #[track_caller]
    fn assert_layout_done(&self, child: &WidgetPod<impl Widget + ?Sized>, method_name: &str) {
        if self.get_child_state(child).needs_layout {
            debug_panic!(
                "Error in {}: trying to call '{}' with child '{}' {} before computing its layout",
                self.widget_id(),
                method_name,
                self.get_child_dyn(child).short_type_name(),
                child.id(),
            );
        }
    }

    #[track_caller]
    fn assert_placed(&self, child: &WidgetPod<impl Widget + ?Sized>, method_name: &str) {
        if self.get_child_state(child).is_expecting_place_child_call {
            debug_panic!(
                "Error in {}: trying to call '{}' with child '{}' {} before placing it",
                self.widget_id(),
                method_name,
                self.get_child_dyn(child).short_type_name(),
                child.id(),
            );
        }
    }

    /// Compute the layout of a child widget.
    ///
    /// Container widgets must call this on every child as part of
    /// their [`layout`] method.
    ///
    /// [`layout`]: Widget::layout
    pub fn run_layout(
        &mut self,
        child: &mut WidgetPod<impl Widget + ?Sized>,
        bc: &BoxConstraints,
    ) -> Size {
        let id = child.id();
        let node = self.children.item_mut(id).unwrap();

        let new_size = run_layout_on(self.global_state, self.default_properties, node, bc);

        let state_mut = &mut self.children.item_mut(id).unwrap().item.state;
        self.widget_state.merge_up(state_mut);

        new_size
    }

    /// Set the position of a child widget, in the parent's coordinate space.
    /// This will affect the parent's display rect.
    ///
    /// Container widgets must call this method with each non-stashed child in their
    /// layout method, after calling `ctx.run_layout(child, bc)`.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`LayoutCtx::run_layout`] has not been called yet for
    /// the child.
    #[track_caller]
    pub fn place_child(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>, origin: Point) {
        self.assert_layout_done(child, "place_child");
        if origin.x.is_nan()
            || origin.x.is_infinite()
            || origin.y.is_nan()
            || origin.y.is_infinite()
        {
            debug_panic!(
                "Error in {}: trying to call 'place_child' with child '{}' {} with invalid origin {:?}",
                self.widget_id(),
                self.get_child_dyn(child).short_type_name(),
                child.id(),
                origin,
            );
        }
        if origin != self.get_child_state_mut(child).origin {
            self.get_child_state_mut(child).origin = origin;
            self.get_child_state_mut(child).transform_changed = true;
        }
        self.get_child_state_mut(child)
            .is_expecting_place_child_call = false;

        self.widget_state.local_paint_rect = self
            .widget_state
            .local_paint_rect
            .union(self.get_child_state(child).paint_rect());
    }

    /// Set explicit paint [`Insets`] for this widget.
    ///
    /// You are not required to set explicit paint bounds unless you need
    /// to paint outside of your layout bounds. In this case, the argument
    /// should be an [`Insets`] struct that indicates where your widget
    /// needs to overpaint, relative to its bounds.
    pub fn set_paint_insets(&mut self, insets: impl Into<Insets>) {
        let insets = insets.into();
        self.widget_state.paint_insets = insets.nonnegative();
    }

    // TODO - This is currently redundant with the code in LayoutCtx::place_child
    /// Given a child and its parent's size, determine the
    /// appropriate paint `Insets` for the parent.
    ///
    /// This is a convenience method; it allows the parent to correctly
    /// propagate a child's desired paint rect, if it extends beyond the bounds
    /// of the parent's layout rect.
    ///
    /// ## Panics
    ///
    /// This method will panic if the child's [`layout()`](LayoutCtx::run_layout) method has not been called yet
    /// and if [`LayoutCtx::place_child()`] has not been called for the child.
    #[track_caller]
    pub fn compute_insets_from_child(
        &mut self,
        child: &WidgetPod<impl Widget + ?Sized>,
        my_size: Size,
    ) -> Insets {
        self.assert_layout_done(child, "compute_insets_from_child");
        self.assert_placed(child, "compute_insets_from_child");
        let parent_bounds = my_size.to_rect();
        let union_paint_rect = self
            .get_child_state(child)
            .paint_rect()
            .union(parent_bounds);
        union_paint_rect - parent_bounds
    }

    /// Set an explicit baseline position for this widget.
    ///
    /// The baseline position is used to align widgets that contain text,
    /// such as buttons, labels, and other controls. It may also be used
    /// by other widgets that are opinionated about how they are aligned
    /// relative to neighbouring text, such as switches or checkboxes.
    ///
    /// The provided value should be the distance from the *bottom* of the
    /// widget to the baseline.
    pub fn set_baseline_offset(&mut self, baseline: f64) {
        self.widget_state.baseline_offset = baseline;
    }

    /// Returns whether this widget needs to call [`LayoutCtx::run_layout`].
    pub fn needs_layout(&self) -> bool {
        self.widget_state.needs_layout
    }

    /// Returns whether a child of this widget needs to call [`LayoutCtx::run_layout`].
    pub fn child_needs_layout(&self, child: &WidgetPod<impl Widget + ?Sized>) -> bool {
        self.get_child_state(child).needs_layout
    }

    /// The distance from the bottom of the given widget to the baseline.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`LayoutCtx::run_layout`] has not been called yet for
    /// the child.
    #[track_caller]
    pub fn child_baseline_offset(&self, child: &WidgetPod<impl Widget + ?Sized>) -> f64 {
        self.assert_layout_done(child, "child_baseline_offset");
        self.get_child_state(child).baseline_offset
    }

    // TODO - Remove (used in Flex)
    #[doc(hidden)]
    #[track_caller]
    pub fn child_layout_rect(&self, child: &WidgetPod<impl Widget + ?Sized>) -> Rect {
        self.assert_layout_done(child, "child_layout_rect");
        self.assert_placed(child, "child_layout_rect");
        self.get_child_state(child).layout_rect()
    }

    /// Get the given child's paint rect.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`LayoutCtx::run_layout`] and [`LayoutCtx::place_child`]
    /// have not been called yet for the child.
    #[track_caller]
    pub fn child_paint_rect(&self, child: &WidgetPod<impl Widget + ?Sized>) -> Rect {
        self.assert_layout_done(child, "child_paint_rect");
        self.assert_placed(child, "child_paint_rect");
        self.get_child_state(child).paint_rect()
    }

    /// Get the given child's size.
    ///
    /// ## Panics
    ///
    /// This method will panic if [`LayoutCtx::run_layout`] has not been called yet for
    /// the child.
    #[track_caller]
    pub fn child_size(&self, child: &WidgetPod<impl Widget + ?Sized>) -> Size {
        self.assert_layout_done(child, "child_size");
        self.get_child_state(child).size
    }

    /// Gives the widget a clip path.
    ///
    /// A widget's clip path will have two effects:
    /// - It serves as a mask for painting operations of the widget's children (*not* the widget itself).
    /// - Pointer events must be inside that path to reach the widget's children.
    pub fn set_clip_path(&mut self, path: Rect) {
        // We intentionally always log this because clip paths are:
        // 1) Relatively rare in the tree
        // 2) An easy potential source of items not being visible when expected
        trace!("set_clip_path {path:?}");
        self.widget_state.clip_path = Some(path);
        // TODO - Updating the clip path may have
        // other knock-on effects we'd need to document.
        self.widget_state.request_accessibility = true;
        self.widget_state.needs_accessibility = true;
        self.widget_state.needs_paint = true;
    }

    /// Remove the widget's clip path.
    ///
    /// See [`LayoutCtx::set_clip_path`] for details.
    pub fn clear_clip_path(&mut self) {
        trace!("clear_clip_path");
        self.widget_state.clip_path = None;
        // TODO - Updating the clip path may have
        // other knock-on effects we'd need to document.
        self.widget_state.request_accessibility = true;
        self.widget_state.needs_accessibility = true;
        self.widget_state.needs_paint = true;
    }

    #[doc(hidden)]
    /// Return the widget's size at the beginning of the layout pass.
    ///
    /// **TODO** This method should be removed after the layout refactor.
    pub fn old_size(&self) -> Size {
        self.widget_state.size
    }
}

impl ComposeCtx<'_> {
    // TODO - Remove?
    /// Returns whether [`Widget::compose`] will be called on this widget.
    pub fn needs_compose(&self) -> bool {
        self.widget_state.needs_compose
    }

    /// Set the scroll translation for the child widget.
    ///
    /// The translation is applied on top of the position from [`LayoutCtx::place_child`].
    pub fn set_child_scroll_translation(
        &mut self,
        child: &mut WidgetPod<impl Widget + ?Sized>,
        translation: Vec2,
    ) {
        if translation.x.is_nan()
            || translation.x.is_infinite()
            || translation.y.is_nan()
            || translation.y.is_infinite()
        {
            debug_panic!(
                "Error in {}: trying to call 'set_child_scroll_translation' with child '{}' {} with invalid translation {:?}",
                self.widget_id(),
                self.get_child_dyn(child).short_type_name(),
                child.id(),
                translation,
            );
        }
        let child = self.get_child_state_mut(child);
        if translation != child.scroll_translation {
            child.scroll_translation = translation;
            child.transform_changed = true;
        }
    }
}

// --- MARK: GET LAYOUT
// Methods on all context types except LayoutCtx
// These methods access layout info calculated during the layout pass.
impl_context_method!(
    MutateCtx<'_>,
    QueryCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    ComposeCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// The layout size.
        ///
        /// This is the layout size returned by the [`layout`] method on the previous
        /// layout pass.
        ///
        /// [`layout`]: Widget::layout
        pub fn size(&self) -> Size {
            self.widget_state.size
        }

        // TODO - Remove. Currently only used in tests.
        #[doc(hidden)]
        pub fn local_layout_rect(&self) -> Rect {
            self.widget_state.layout_rect()
        }

        /// The offset of the baseline relative to the bottom of the widget.
        pub fn baseline_offset(&self) -> f64 {
            self.widget_state.baseline_offset
        }

        /// The origin of the widget in window coordinates, relative to the top left corner of the
        /// content area.
        pub fn window_origin(&self) -> Point {
            self.widget_state.window_origin()
        }

        /// Global transform of this widget in the window coordinate space.
        ///
        /// Computed from all `transform` and `scroll_translation` values from this to the root widget.
        pub fn window_transform(&self) -> Affine {
            self.widget_state.window_transform
        }

        /// The bounding rect of the widget in window coordinates.
        ///
        /// See [bounding rect documentation](crate::doc::masonry_concepts#bounding-rect)
        /// for details.
        pub fn bounding_rect(&self) -> Rect {
            self.widget_state.bounding_rect()
        }

        // TODO - Remove? See above.
        /// The paint rect of the widget.
        ///
        /// Covers the area we expect to be invalidated when the widget is painted.
        pub fn paint_rect(&self) -> Rect {
            self.widget_state.paint_rect()
        }

        /// The clip path of the widget, if any was set.
        ///
        /// For more information, see
        /// [`LayoutCtx::set_clip_path`](crate::core::LayoutCtx::set_clip_path).
        pub fn clip_path(&self) -> Option<Rect> {
            self.widget_state.clip_path
        }

        /// Convert a point from the widget's coordinate space to the window's.
        ///
        /// The returned point is relative to the content area; it excludes window chrome.
        pub fn to_window(&self, widget_point: Point) -> Point {
            self.widget_state.window_transform * widget_point
        }
    }
);

impl_context_method!(AccessCtx<'_>, EventCtx<'_>, PaintCtx<'_>, {
    /// Get DPI scaling factor.
    ///
    /// This is not required for most widgets, and should be used only for precise
    /// rendering, such as rendering single pixel lines or selecting image variants.
    /// This is currently only provided in the render stages, as these are the only passes which
    /// are re-run when the scale factor changes, except [`EventCtx`] where it is necessary to
    /// translate pointer events which are currently in physical coordinates.
    ///
    /// Note that accessibility nodes and paint results will automatically be scaled by Masonry.
    /// This also doesn't account for the widget's current transform, which cannot currently be
    /// accessed by widgets directly.
    pub fn get_scale_factor(&self) -> f64 {
        self.global_state.scale_factor
    }
});

// Methods on all context types
// Access status information (hovered/pointer captured/disabled/etc).
impl_context_method!(
    MutateCtx<'_>,
    QueryCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// The "hovered" status of a widget.
        ///
        /// A widget is "hovered" when a pointer is hovering over it. Widgets will
        /// often change their appearance as a visual indication that they
        /// will respond to pointer (usually mouse) interaction.
        ///
        /// If the pointer is [captured], then only the capturing widget can have hovered
        /// status. If the pointer is captured but not hovering over the captured
        /// widget, then no widget has the hovered status.
        ///
        /// [captured]: crate::doc::masonry_concepts#pointer-capture
        pub fn is_hovered(&self) -> bool {
            self.widget_state.is_hovered
        }

        /// Whether this widget or any of its descendants are hovered.
        ///
        /// To check if only this specific widget is hovered use [`is_hovered`](Self::is_hovered).
        pub fn has_hovered(&self) -> bool {
            self.widget_state.has_hovered
        }

        /// The "active" status of a widget.
        ///
        /// A widget is "active" when the user is "actively" interacting with it.
        /// Currently, a widget is determined to be active when it has [captured] a pointer,
        /// but this may change in the future to account for e.g. keyboard interactions.
        ///
        /// [captured]: crate::doc::masonry_concepts#pointer-capture
        pub fn is_active(&self) -> bool {
            self.widget_state.is_active
        }

        /// Whether this widget or any of its descendants are active.
        ///
        /// To check if only this specific widget is active use [`is_active`](Self::is_active).
        pub fn has_active(&self) -> bool {
            self.widget_state.has_active
        }

        /// Whether a pointer is [captured] by this widget.
        ///
        /// The pointer will usually be the mouse. In future versions, this
        /// function will take a pointer id as input to test a specific pointer.
        ///
        /// [captured]: crate::doc::masonry_concepts#pointer-capture
        pub fn is_pointer_capture_target(&self) -> bool {
            self.global_state.pointer_capture_target == Some(self.widget_state.id)
        }

        /// The [text focus] status of a widget.
        ///
        /// The focused widget is the one that receives keyboard events.
        ///
        /// Returns `true` if this specific widget is focused.
        /// To check if any descendants are focused use [`has_focus_target`].
        ///
        /// [text focus]: crate::doc::masonry_concepts#text-focus
        /// [`has_focus_target`]: Self::has_focus_target
        pub fn is_focus_target(&self) -> bool {
            self.global_state.focused_widget == Some(self.widget_id())
        }

        /// Whether this widget or any of its descendants are focused.
        ///
        /// To check if only this specific widget is focused use [`is_focus_target`](Self::is_focus_target).
        pub fn has_focus_target(&self) -> bool {
            self.widget_state.has_focus_target
        }

        /// Whether the window is focused.
        pub fn is_window_focused(&self) -> bool {
            self.global_state.window_focused
        }

        /// Whether this widget gets pointer events and hovered status.
        pub fn accepts_pointer_interaction(&self) -> bool {
            self.widget_state.accepts_pointer_interaction
        }

        /// Whether this widget gets text focus.
        pub fn accepts_focus(&self) -> bool {
            self.widget_state.accepts_focus
        }

        /// Whether this widget gets IME events.
        pub fn accepts_text_input(&self) -> bool {
            self.widget_state.accepts_text_input
        }

        /// The disabled state of a widget.
        ///
        /// Returns `true` if this widget or any of its ancestors is explicitly disabled.
        /// To make this widget explicitly disabled use [`set_disabled`].
        ///
        /// Disabled means that this widget should not change the state of the application. What
        /// that means is not entirely clear but in any case it should not change its data. Therefore
        /// others can use this as a safety mechanism to prevent the application from entering an
        /// illegal state.
        /// For an example the decrease button of a counter of type `usize` should be disabled if the
        /// value is `0`.
        ///
        /// [`set_disabled`]: EventCtx::set_disabled
        pub fn is_disabled(&self) -> bool {
            self.widget_state.is_disabled
        }

        /// Check is widget is [stashed]().
        ///
        /// [stashed]: crate::doc::masonry_concepts#stashed
        pub fn is_stashed(&self) -> bool {
            self.widget_state.is_stashed
        }
    }
);

// --- MARK: UPDATE FLAGS
// Methods on MutateCtx, EventCtx, and UpdateCtx
impl_context_method!(MutateCtx<'_>, EventCtx<'_>, UpdateCtx<'_>, {
    /// Request a [`paint`](crate::core::Widget::paint) and an [`accessibility`](crate::core::Widget::accessibility) pass.
    pub fn request_render(&mut self) {
        trace!("request_render");
        self.widget_state.request_paint = true;
        self.widget_state.needs_paint = true;
        self.widget_state.needs_accessibility = true;
        self.widget_state.request_accessibility = true;
    }

    /// Request a [`paint`](crate::core::Widget::paint) pass.
    ///
    /// Unlike [`request_render`](Self::request_render), this does not request an [`accessibility`](crate::core::Widget::accessibility) pass.
    /// Use `request_render` unless you're sure an accessibility pass is not needed.
    pub fn request_paint_only(&mut self) {
        trace!("request_paint");
        self.widget_state.request_paint = true;
        self.widget_state.needs_paint = true;
    }

    /// Request an [`accessibility`](crate::core::Widget::accessibility) pass.
    ///
    /// This doesn't request a [`paint`](crate::core::Widget::paint) pass.
    /// If you want to request both an accessibility pass and a paint pass, use [`request_render`](Self::request_render).
    pub fn request_accessibility_update(&mut self) {
        trace!("request_accessibility_update");
        self.widget_state.needs_accessibility = true;
        self.widget_state.request_accessibility = true;
    }

    /// Request a [`layout`] pass.
    ///
    /// Call this method if the widget has changed in a way that requires a layout pass.
    ///
    /// [`layout`]: crate::core::Widget::layout
    pub fn request_layout(&mut self) {
        trace!("request_layout");
        self.widget_state.request_layout = true;
        self.widget_state.needs_layout = true;
    }

    // TODO - Document better
    /// Request a [`compose`] pass.
    ///
    /// The compose pass is often cheaper than the layout pass, because it can only transform individual widgets' position.
    ///
    /// [`compose`]: crate::core::Widget::compose
    pub fn request_compose(&mut self) {
        trace!("request_compose");
        self.widget_state.needs_compose = true;
        self.widget_state.request_compose = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        trace!("request_anim_frame");
        self.widget_state.request_anim = true;
        self.widget_state.needs_anim = true;
    }

    /// Notifies Masonry that the cursor returned by [`Widget::get_cursor`] has changed.
    ///
    /// This is mostly meant for cases where the cursor changes even if the pointer doesn't
    /// move, because the nature of the widget has changed somehow.
    pub fn request_cursor_icon_change(&mut self) {
        trace!("request_cursor_icon_change");
        self.global_state.needs_pointer_pass = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        trace!("children_changed");
        self.widget_state.children_changed = true;
        self.widget_state.needs_update_focus_chain = true;
        self.request_layout();
    }

    /// Indicate that a child is about to be removed from the tree.
    ///
    /// Container widgets should avoid dropping `WidgetPod`s. Instead, they should
    /// pass them to this method.
    pub fn remove_child(&mut self, child: WidgetPod<impl Widget + ?Sized>) {
        // TODO - Send recursive event to child
        let id = child.id();
        let _ = self
            .children
            .remove(id)
            .expect("remove_child: child not found");
        self.global_state.scenes.remove(&child.id());

        self.children_changed();
    }

    /// Set the disabled state for this widget.
    ///
    /// Setting this to `false` does not mean a widget is not still disabled; for instance it may
    /// still be disabled by an ancestor. See [`is_disabled`] for more information.
    ///
    /// [`is_disabled`]: EventCtx::is_disabled
    pub fn set_disabled(&mut self, disabled: bool) {
        self.widget_state.needs_update_disabled = true;
        self.widget_state.is_explicitly_disabled = disabled;
    }

    /// Set the transform for this widget.
    ///
    /// It behaves similarly as CSS transforms
    pub fn set_transform(&mut self, transform: Affine) {
        self.widget_state.transform = transform;
        self.widget_state.transform_changed = true;
        self.request_compose();
    }
});

// --- MARK: OTHER METHODS
// Methods on mutable context types
impl_context_method!(
    MutateCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    {
        // TODO - Remove from LayoutCtx/ComposeCtx
        /// Mark child widget as stashed.
        ///
        /// If `stashed` is true, the child will not be painted or listed in the accessibility tree.
        ///
        /// This will *not* trigger a layout pass.
        pub fn set_stashed(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>, stashed: bool) {
            let child_state = self.get_child_state_mut(child);
            // Stashing is generally a property derived from the parent widget's state
            // (rather than set imperatively), so it is likely to be set as part of passes.
            // Therefore, we avoid re-running the update_stashed_pass in most cases.
            if child_state.is_explicitly_stashed != stashed {
                child_state.needs_update_stashed = true;
                child_state.is_explicitly_stashed = stashed;
            }
        }

        // TODO - Remove from MutateCtx?
        /// Queue a callback that will be called with a [`WidgetMut`] for this widget.
        ///
        /// The callbacks will be run in the order they were submitted during the mutate pass.
        pub fn mutate_self_later(
            &mut self,
            f: impl FnOnce(WidgetMut<'_, dyn Widget>) + Send + 'static,
        ) {
            let callback = MutateCallback {
                id: self.widget_state.id,
                callback: Box::new(f),
            };
            self.global_state.mutate_callbacks.push(callback);
        }

        /// Queue a callback that will be called with a [`WidgetMut`] for the given child widget.
        ///
        /// The callbacks will be run in the order they were submitted during the mutate pass.
        pub fn mutate_later<W: Widget + FromDynWidget + ?Sized>(
            &mut self,
            child: &mut WidgetPod<W>,
            f: impl FnOnce(WidgetMut<'_, W>) + Send + 'static,
        ) {
            let callback = MutateCallback {
                id: child.id(),
                callback: Box::new(|mut widget_mut| f(widget_mut.downcast())),
            };
            self.global_state.mutate_callbacks.push(callback);
        }

        /// Submit an Action, which indicates that this widget requires something be handled
        /// by the application, such as user input.
        ///
        /// For further details see [`ErasedAction`].
        /// If you have an already boxed action, prefer `submit_erased_action`.s
        pub fn submit_action(&mut self, action: impl AnyDebug + Send) {
            self.submit_erased_action(Box::new(action));
        }

        /// Submit an already boxed action.
        ///
        /// See `submit_action` for more details.
        pub fn submit_erased_action(&mut self, action: ErasedAction) {
            trace!("submit_action");
            self.global_state
                .emit_signal(RenderRootSignal::Action(action, self.widget_state.id));
        }

        /// Set the IME cursor area.
        ///
        /// When this widget is [focused] and [accepts text input], the reported IME area is sent
        /// to the platform. The area can be used by the platform to, for example, place a
        /// candidate box near that area, while ensuring the area is not obscured.
        ///
        /// If no IME area is set, the platform will use the widget's layout rect.
        ///
        /// [focused]: EventCtx::request_focus
        /// [accepts text input]: Widget::accepts_text_input
        pub fn set_ime_area(&mut self, ime_area: Rect) {
            self.widget_state.ime_area = Some(ime_area);
        }

        /// Remove the IME cursor area.
        ///
        /// See [`LayoutCtx::set_ime_area`](LayoutCtx::set_ime_area) for more details.
        pub fn clear_ime_area(&mut self) {
            self.widget_state.ime_area = None;
        }

        /// Start a window drag.
        ///
        /// Moves the window with the left mouse button until the button is released.
        pub fn drag_window(&mut self) {
            trace!("drag_window");
            self.global_state.emit_signal(RenderRootSignal::DragWindow);
        }

        /// Start a window resize.
        ///
        /// Resizes the window with the left mouse button until the button is released.
        pub fn drag_resize_window(&mut self, direction: ResizeDirection) {
            trace!("drag_resize_window");
            self.global_state
                .emit_signal(RenderRootSignal::DragResizeWindow(direction));
        }

        /// Toggle the maximized state of the window.
        pub fn toggle_maximized(&mut self) {
            trace!("toggle_maximized");
            self.global_state
                .emit_signal(RenderRootSignal::ToggleMaximized);
        }

        /// Minimize the window.
        pub fn minimize(&mut self) {
            trace!("minimize");
            self.global_state.emit_signal(RenderRootSignal::Minimize);
        }

        /// Exit the application.
        pub fn exit(&mut self) {
            trace!("exit");
            self.global_state.emit_signal(RenderRootSignal::Exit);
        }

        /// Show the window menu at a specified position.
        pub fn show_window_menu(&mut self, position: LogicalPosition<f64>) {
            trace!("show_window_menu");
            self.global_state
                .emit_signal(RenderRootSignal::ShowWindowMenu(position));
        }
    }
);

impl RegisterCtx<'_> {
    /// Register a child widget.
    ///
    /// Container widgets should call this on all their children in
    /// their implementation of [`Widget::register_children`].
    pub fn register_child(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>) {
        let Some(NewWidget {
            widget,
            id,
            options,
            properties,
        }) = child.take_inner()
        else {
            return;
        };

        #[cfg(debug_assertions)]
        {
            self.registered_ids.push(id);
        }

        let state = WidgetState::new(id, widget.short_type_name(), options);

        let node = WidgetArenaNode {
            widget: widget.as_box_dyn(),
            state,
            properties: properties.map,
        };
        self.children.insert(id, node);
    }
}

// --- MARK: DEBUG PAINT
impl PaintCtx<'_> {
    /// Whether debug paint is enabled.
    ///
    /// If this property is set, your widget may draw additional debug information
    /// (such as the position of the text baseline).
    /// These should normally use the [debug color][Self::debug_color] for this widget.
    /// Please note that when debug painting is enabled, each widget's layout boundaries are
    /// outlined by Masonry, so you should avoid duplicating that.
    ///
    /// Debug paint can be enabled by setting the environment variable `MASONRY_DEBUG_PAINT`.
    pub fn debug_paint_enabled(&self) -> bool {
        self.debug_paint
    }

    /// A color used for debug painting in this widget.
    ///
    /// This is normally used to paint additional debugging information
    /// when debug paint is enabled, see [`Self::debug_paint_enabled`].
    pub fn debug_color(&self) -> Color {
        get_debug_color(self.widget_id().to_raw())
    }
}

// --- MARK: RAW WRAPPERS
macro_rules! impl_get_raw {
    ($SomeCtx:tt) => {
        impl<'s> $SomeCtx<'s> {
            /// Get a child context and a raw shared reference to a child widget.
            ///
            /// The child context can be used to call context methods on behalf of the
            /// child widget.
            pub fn get_raw_ref<'a, 'r, Child: Widget + FromDynWidget + ?Sized>(
                &'a mut self,
                child: &'a mut WidgetPod<Child>,
            ) -> RawWrapper<'r, $SomeCtx<'r>, Child>
            where
                'a: 'r,
                's: 'r,
            {
                let child_mut = self
                    .children
                    .item_mut(child.id())
                    .expect("get_raw_ref: child not found");
                #[allow(
                    clippy::needless_update,
                    reason = "May be needless in some macro invocations"
                )]
                let child_ctx = $SomeCtx {
                    widget_state: &mut child_mut.item.state,
                    global_state: self.global_state,
                    children: child_mut.children,
                    ..*self
                };
                RawWrapper {
                    ctx: child_ctx,
                    widget: Child::from_dyn(&*child_mut.item.widget).unwrap(),
                }
            }

            /// Get a raw mutable reference to a child widget.
            ///
            /// See documentation for [`AllowRawMut`] for more details.
            pub fn get_raw_mut<'a, 'r, Child: Widget + FromDynWidget + AllowRawMut + ?Sized>(
                &'a mut self,
                child: &'a mut WidgetPod<Child>,
            ) -> RawWrapperMut<'r, $SomeCtx<'r>, Child>
            where
                'a: 'r,
                's: 'r,
            {
                let child_mut = self
                    .children
                    .item_mut(child.id())
                    .expect("get_raw_mut: child not found");
                #[allow(
                    clippy::needless_update,
                    reason = "May be needless in some macro invocations"
                )]
                let child_ctx = $SomeCtx {
                    widget_state: &mut child_mut.item.state,
                    global_state: self.global_state,
                    children: child_mut.children,
                    ..*self
                };
                RawWrapperMut {
                    parent_widget_state: &mut self.widget_state,
                    ctx: child_ctx,
                    widget: Child::from_dyn_mut(&mut *child_mut.item.widget).unwrap(),
                }
            }
        }
    };
}

impl_get_raw!(EventCtx);
impl_get_raw!(UpdateCtx);
impl_get_raw!(LayoutCtx);

#[allow(missing_docs, reason = "RawWrapper is likely to be reworked")]
impl<'s> AccessCtx<'s> {
    pub fn get_raw_ref<'a, 'r, Child: Widget + FromDynWidget + ?Sized>(
        &'a mut self,
        child: &'a WidgetPod<Child>,
    ) -> RawWrapper<'r, AccessCtx<'r>, Child>
    where
        'a: 'r,
        's: 'r,
    {
        let child_mut = self
            .children
            .item_mut(child.id())
            .expect("get_raw_ref: child not found");
        let child_ctx = AccessCtx {
            widget_state: &mut child_mut.item.state,
            global_state: self.global_state,
            children: child_mut.children,
            tree_update: self.tree_update,
            rebuild_all: self.rebuild_all,
        };
        RawWrapper {
            ctx: child_ctx,
            widget: Child::from_dyn(&*child_mut.item.widget).unwrap(),
        }
    }
}

#[allow(missing_docs, reason = "RawWrapper is likely to be reworked")]
pub struct RawWrapper<'a, Ctx, W: ?Sized> {
    ctx: Ctx,
    widget: &'a W,
}

#[allow(missing_docs, reason = "RawWrapper is likely to be reworked")]
pub struct RawWrapperMut<'a, Ctx: IsContext, W: ?Sized> {
    parent_widget_state: &'a mut WidgetState,
    ctx: Ctx,
    widget: &'a mut W,
}

#[allow(missing_docs, reason = "RawWrapper is likely to be reworked")]
impl<Ctx, W: ?Sized> RawWrapper<'_, Ctx, W> {
    pub fn widget(&self) -> &W {
        self.widget
    }

    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }
}

#[allow(missing_docs, reason = "RawWrapper is likely to be reworked")]
impl<Ctx: IsContext, W: ?Sized> RawWrapperMut<'_, Ctx, W> {
    pub fn widget(&mut self) -> &mut W {
        self.widget
    }

    pub fn ctx(&mut self) -> &mut Ctx {
        &mut self.ctx
    }
}

impl<Ctx: IsContext, W: ?Sized> Drop for RawWrapperMut<'_, Ctx, W> {
    fn drop(&mut self) {
        self.parent_widget_state
            .merge_up(self.ctx.get_widget_state());
    }
}

mod private {
    #[allow(
        unnameable_types,
        reason = "see https://predr.ag/blog/definitive-guide-to-sealed-traits-in-rust/"
    )]
    pub trait Sealed {}
}

// TODO - Rethink RawWrapper API
// We're exporting a trait with a method that returns a private type.
// It's mostly fine because the trait is sealed anyway, but it's not great for documentation.

#[allow(
    private_interfaces,
    reason = "see https://predr.ag/blog/definitive-guide-to-sealed-traits-in-rust/"
)]
#[allow(missing_docs, reason = "RawWrapper is likely to be reworked")]
pub trait IsContext: private::Sealed {
    fn get_widget_state(&mut self) -> &mut WidgetState;
}

macro_rules! impl_context_trait {
    ($SomeCtx:tt) => {
        impl private::Sealed for $SomeCtx<'_> {}

        #[allow(
            private_interfaces,
            reason = "see https://predr.ag/blog/definitive-guide-to-sealed-traits-in-rust/"
        )]
        impl IsContext for $SomeCtx<'_> {
            fn get_widget_state(&mut self) -> &mut WidgetState {
                self.widget_state
            }
        }
    };
}

impl_context_trait!(EventCtx);
impl_context_trait!(UpdateCtx);
impl_context_trait!(LayoutCtx);
