// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! The context types that are passed into various widget methods.

use std::any::Any;
use std::collections::hash_map::Entry;

use accesskit::{NodeId, TreeUpdate};
use anymore::AnyDebug;
use dpi::{LogicalPosition, PhysicalPosition};
use parley::{FontContext, LayoutContext};
use tracing::{trace, warn};
use tree_arena::{ArenaMut, ArenaMutList, ArenaRefList};

use crate::app::{MutateCallback, RenderRootSignal, RenderRootState};
use crate::core::{
    AllowRawMut, BrushIndex, DefaultProperties, ErasedAction, FromDynWidget, LayerType, NewWidget,
    PropertiesMut, PropertiesRef, ResizeDirection, Widget, WidgetArenaNode, WidgetId, WidgetMut,
    WidgetPod, WidgetRef, WidgetState,
};
use crate::kurbo::{Affine, Axis, Insets, Point, Rect, Size, Vec2};
use crate::layout::{LayoutSize, LenDef, SizeDef};
use crate::passes::layout::{place_widget, resolve_length, resolve_size, run_layout_on};
use crate::peniko::Color;
use crate::util::{TypeSet, get_debug_color};

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
    pub(crate) changed_properties: &'a mut TypeSet,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) default_properties: &'a DefaultProperties,
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
    pub(crate) default_properties: &'a DefaultProperties,
}

/// A context given when calling another context's `get_raw_mut()` method.
pub struct RawCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) parent_widget_state: &'a mut WidgetState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) default_properties: &'a DefaultProperties,
}

/// A context provided to event-handling [`Widget`] methods.
pub struct EventCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) default_properties: &'a DefaultProperties,
    pub(crate) target: WidgetId,
    pub(crate) allow_pointer_capture: bool,
    pub(crate) is_handled: bool,
}

/// A context provided to the [`Widget::register_children`] method.
pub struct RegisterCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    #[cfg(debug_assertions)]
    pub(crate) registered_ids: Vec<WidgetId>,
}

/// A context provided to the [`Widget::update`] method.
pub struct UpdateCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) default_properties: &'a DefaultProperties,
}

/// A context provided to [`Widget::measure`] methods.
pub struct MeasureCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) default_properties: &'a DefaultProperties,
    pub(crate) auto_length: LenDef,
    pub(crate) context_size: LayoutSize,
    pub(crate) cache_result: bool,
}

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
    pub(crate) default_properties: &'a DefaultProperties,
}

/// A context passed to [`Widget::paint`] method.
pub struct PaintCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
}

/// A context passed to [`Widget::accessibility`] method.
pub struct AccessCtx<'a> {
    pub(crate) global_state: &'a mut RenderRootState,
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) children: ArenaMutList<'a, WidgetArenaNode>,
    pub(crate) tree_update: &'a mut TreeUpdate,
}

// --- MARK: GETTERS
// Methods for all context types
impl_context_method!(
    MutateCtx<'_>,
    QueryCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    MeasureCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    RawCtx<'_>,
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

        #[allow(dead_code, reason = "Copy-pasted for some types that don't need it")]
        /// Returns the local transform of this widget.
        ///
        /// This transform is used during the mapping of this widget's border-box coordinate space
        /// to the parent's border-box coordinate space.
        ///
        /// When calculating the effective border-box of this widget, first this transform
        /// will be applied and then `scroll_translation` and `origin` applied on top.
        pub fn transform(&self) -> Affine {
            self.widget_state.transform
        }
    }
);

impl_context_method!(
    MutateCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    MeasureCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    RawCtx<'_>,
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
    /// Returns a [`WidgetMut`] to a child widget.
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
            changed_properties: &mut node_mut.item.changed_properties,
            children: node_mut.children,
            default_properties: self.default_properties,
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
            changed_properties: self.changed_properties,
            children: self.children.reborrow_mut(),
            default_properties: self.default_properties,
        }
    }

    pub(crate) fn update_mut(&mut self) -> UpdateCtx<'_> {
        UpdateCtx {
            global_state: self.global_state,
            widget_state: self.widget_state,
            children: self.children.reborrow_mut(),
            default_properties: self.default_properties,
        }
    }

    /// Returns `true` if the [local transform] of this widget has been modified since
    /// the last time this widget's transformation was resolved.
    ///
    /// This is exposed for Xilem, and is more likely to change or be removed
    /// in major releases of Masonry.
    ///
    /// [local transform]: Self::transform
    pub fn transform_has_changed(&self) -> bool {
        self.widget_state.transform_changed
    }
}

// --- MARK: WIDGET_REF
// Methods to get a child WidgetRef from a parent.
impl<'w> QueryCtx<'w> {
    /// Returns a [`WidgetRef`] to a child widget.
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
            default_properties: self.default_properties,
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
    MeasureCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    RawCtx<'_>,
    {
        /// Returns the Parley contexts needed to build and paint text sections.
        ///
        /// Most users should embed a pre-built label widget
        /// (such as `masonry::widgets::Label`)
        /// as a child for non-interactive text.
        /// These contexts could however be useful for custom text editing, such as for rich text editing.
        ///
        /// Any cached text layouts should be invalidated in the layout pass when [`Self::fonts_changed`]
        /// returns `true`.
        pub fn text_contexts(&mut self) -> (&mut FontContext, &mut LayoutContext<BrushIndex>) {
            (
                &mut self.global_state.font_context,
                &mut self.global_state.text_layout_context,
            )
        }

        /// Whether the set of loaded fonts has changed since layout was most recently called.
        ///
        /// Any cached text layouts should be invalidated in the layout pass when this is `true`.
        pub fn fonts_changed(&mut self) -> bool {
            self.global_state.fonts_changed
        }
    }
);

// --- MARK: EVENT HANDLING
impl EventCtx<'_> {
    /// Captures the pointer in the current widget.
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

    /// Releases the pointer previously [captured] through [`capture_pointer`].
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

    /// Sends a signal to parent widgets to scroll this widget's border-box into view.
    pub fn request_scroll_to_this(&mut self) {
        let rect = self.widget_state.border_box_size().to_rect();
        self.global_state
            .scroll_request_targets
            .push((self.widget_state.id, rect));
    }

    /// Sends a signal to parent widgets to scroll the provided `rect` into view.
    ///
    /// The `rect` must be in this widget's content-box coordinate space.
    pub fn request_scroll_to(&mut self, rect: Rect) {
        // Convert from this widget's content-box space to border-box space.
        let rect = rect + self.widget_state.border_box_translation();
        self.global_state
            .scroll_request_targets
            .push((self.widget_state.id, rect));
    }

    /// Sets the event as "handled", which stops its propagation to parent
    /// widgets.
    pub fn set_handled(&mut self) {
        trace!("set_handled");
        self.is_handled = true;
    }

    /// Determines whether the event has been handled.
    pub fn is_handled(&self) -> bool {
        self.is_handled
    }

    /// The widget originally targeted by the event.
    ///
    /// This will be different from [`widget_id`](Self::widget_id) during event bubbling.
    pub fn target(&self) -> WidgetId {
        self.target
    }

    /// Requests [text focus].
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

    /// Transfers [text focus] to the widget with the given `WidgetId`.
    ///
    /// [text focus]: crate::doc::masonry_concepts#text-focus
    pub fn set_focus(&mut self, target: WidgetId) {
        trace!("set_focus target={:?}", target);
        self.global_state.next_focused_widget = Some(target);
    }

    /// Gives up [text focus].
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

    /// Converts the given position from the window's coordinate space
    /// to this widget's content-box coordinate space.
    pub fn local_position(&self, p: PhysicalPosition<f64>) -> Point {
        // TODO: Remove this .to_logical() conversion when scale refactor work happens.
        //       https://github.com/linebender/xilem/issues/1264
        let LogicalPosition { x, y } = p.to_logical(self.global_state.scale_factor);
        self.to_local(Point { x, y })
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

    /// Returns an id which is guaranteed not to collide with [`WidgetId`]s or with previous ids returned by this function.
    pub fn next_node_id() -> NodeId {
        // TODO - Return from a pool disjoint from widget ids.
        WidgetId::next().into()
    }
}

// --- MARK: COMPUTE LENGTH
impl_context_method!(MeasureCtx<'_>, LayoutCtx<'_>, {
    /// Computes the `child`'s preferred border-box length on the given `axis`.
    ///
    /// The returned length will be finite, non-negative, and in device pixels.
    ///
    /// Container widgets usually call this method as part of their [`measure`] logic,
    /// to help them calculate their own length on the given `axis`. They call it as part
    /// of their [`layout`] logic if they have already chosen a length for one axis.
    /// Read [`measure`] and [`layout`] docs for more details about those processes.
    ///
    /// `auto_length` specifies the fallback behavior if the child's dimension is [`Dim::Auto`].
    /// If you're calling this from within [`measure`] then you usually want to derive
    /// this from `len_req`, probably using [`LenReq::reduce`], i.e. you would call
    /// `len_req.reduce(used_space_on_this_axis).into()`. However, if you're calling this
    /// from within [`layout`] then you usually want to use use [`LenDef::FitContent`]
    /// to ask the child to fit inside the available space. Sometimes a different fallback
    /// makes more sense, e.g. `Grid` uses [`LenDef::Fixed`] to fall back to the exact
    /// allocated child area size.
    /// `auto_length` values must be finite, non-negative, and in device pixels.
    /// An invalid `auto_length` will fall back to [`LenDef::MaxContent`].
    ///
    /// `context_size` is the size, in device pixels, that is used to resolve relative sizes.
    /// For example [`Ratio(0.5)`] will result in half the context size.
    /// This is usually the container widget's content-box size, i.e. excluding borders and padding.
    /// Examples of exceptions include `Grid` which will provide the child's area size,
    /// i.e. the union of cell sizes that the child occupies, and `Portal` which will provide
    /// its viewport size.
    ///
    /// `cross_length` is the length of the cross axis and is critical information for certain
    /// widgets, e.g. for text max advance or to keep an aspect ratio.
    /// If present, `cross_length` must be finite, non-negative, and in device pixels.
    /// An invalid `cross_length` will fall back to `None`.
    ///
    /// # Panics
    ///
    /// Panics if `auto_length` is non-finite or negative and debug assertions are enabled.
    ///
    /// Panics if `cross_length` is non-finite or negative and debug assertions are enabled.
    ///
    /// [`measure`]: Widget::measure
    /// [`layout`]: Widget::layout
    /// [`Ratio(0.5)`]: crate::layout::Dim::Ratio
    /// [`Dim::Auto`]: crate::layout::Dim
    /// [`LenReq::reduce`]: crate::layout::LenReq::reduce
    pub fn compute_length(
        &mut self,
        child: &mut WidgetPod<impl Widget + ?Sized>,
        auto_length: LenDef,
        context_size: LayoutSize,
        axis: Axis,
        cross_length: Option<f64>,
    ) -> f64 {
        let id = child.id();
        let node = self.children.item_mut(id).unwrap();
        resolve_length(
            self.global_state,
            self.default_properties,
            node,
            auto_length,
            context_size,
            axis,
            cross_length,
        )
    }
});

// --- MARK: MEASURE
impl MeasureCtx<'_> {
    /// Returns the fallback [`LenDef`] of this measurement.
    ///
    /// This is essential information for widgets that want pass through measurements.
    /// That is, this fallback was chosen by widget A for widget B.
    /// Widget B can call this method and then pass it on via [`compute_length`] to widget C.
    /// Widget C will resolve its size with the fallback originally provided by widget A.
    ///
    /// Even easier, though, is to use [`redirect_measurement`].
    ///
    /// Calling `auto_length` will cause the result of this [`measure`] to not be cached.
    /// This is because `auto_length` is not part of the cache key.
    ///
    /// [`measure`]: Widget::measure
    /// [`compute_length`]: Self::compute_length
    /// [`redirect_measurement`]: Self::redirect_measurement
    pub fn auto_length(&mut self) -> LenDef {
        // We're adding a new variable, auto_length, into the measure function,
        // which is not part of the cache key. Hence, we need to not cache.
        self.cache_result = false;
        self.auto_length
    }

    /// Returns the context size of this measurement.
    ///
    /// This is usually the container widget's content-box size, i.e. excluding borders and padding.
    ///
    /// Examples of exceptions include `Grid` which will provide the child's area size,
    /// i.e. the union of cell sizes that the child occupies, and `Portal` which will provide
    /// its viewport size.
    ///
    /// The context size is used to resolve relative lengths, e.g. a width of [`Ratio(0.5)`]
    /// means half of this space. This resolving is done by Masonry and not manually in [`measure`].
    ///
    /// One or both lengths may be missing if they have not been computed yet,
    /// i.e. when the lengths depend on a child's size.
    ///
    /// This is essential information for widgets that want pass through measurements.
    /// That is, this context size was chosen by widget A for widget B.
    /// Widget B can call this method and then pass it on via [`compute_length`] to widget C.
    /// Widget C will resolve its size in relation to widget A.
    ///
    /// Even easier, though, is to use [`redirect_measurement`].
    ///
    /// Calling `context_size` will cause the result of this [`measure`] to not be cached.
    /// This is because `context_size` is not part of the cache key.
    ///
    /// [`measure`]: Widget::measure
    /// [`Ratio(0.5)`]: crate::layout::Dim::Ratio
    /// [`compute_length`]: Self::compute_length
    /// [`redirect_measurement`]: Self::redirect_measurement
    pub fn context_size(&mut self) -> LayoutSize {
        // We're adding a new variable, context_size, into the measure function,
        // which is not part of the cache key. Hence, we need to not cache.
        self.cache_result = false;
        self.context_size
    }

    /// Configures whether this [`measure`] result will be cached.
    ///
    /// Masonry will, by default, cache the results of measurement. The cache key is derived
    /// from `axis`, `len_req`, and `cross_length`. If the widget uses any other data to influence
    /// the result of the measurement, then the widget is responsible for requesting layout
    /// when any of that data changes. For properties, this is handled in [`property_changed`].
    /// For any other data you reference, the exact mechanism of detecting changes is up to you.
    /// If you can't detect changes of the referenced data, disable the cache with this method.
    ///
    /// [`measure`]: Widget::measure
    /// [`property_changed`]: Widget::property_changed
    pub fn cache_result(&mut self, cache_result: bool) {
        self.cache_result = cache_result;
    }

    /// Redirects the measurement request to a `child`.
    ///
    /// This is meant for thin wrapper widgets that want their children measured instead.
    ///
    /// It is a convenience wrapper over [`compute_length`] that automatically configures
    /// `auto_length` and `context_size` to whatever the outer container used. These could also
    /// be manually accessed via [`auto_length`] and [`context_size`] if you're so inclined.
    ///
    /// Calling `redirect_measurement` will cause the result of your [`measure`] to not be cached.
    /// The child's measurement might still be cached, depending on what policy the child chooses.
    /// This is because the redirection introduces new inputs in the form of [`auto_length`]
    /// and [`context_size`] that are not part of the cache key.
    ///
    /// If present, `cross_length` must be finite, non-negative, and in device pixels.
    /// An invalid `cross_length` will fall back to `None`.
    ///
    /// # Panics
    ///
    /// Panics if `cross_length` is non-finite or negative and debug assertions are enabled.
    ///
    /// [`measure`]: Widget::measure
    /// [`compute_length`]: Self::compute_length
    /// [`auto_length`]: Self::auto_length
    /// [`context_size`]: Self::context_size
    pub fn redirect_measurement(
        &mut self,
        child: &mut WidgetPod<impl Widget + ?Sized>,
        axis: Axis,
        cross_length: Option<f64>,
    ) -> f64 {
        // We're adding two new variables, auto_length and context_size, into the measure function,
        // which are not part of the cache key. Hence, we need to not cache.
        self.cache_result = false;
        self.compute_length(
            child,
            self.auto_length,
            self.context_size,
            axis,
            cross_length,
        )
    }
}

// --- MARK: UPDATE LAYOUT
impl LayoutCtx<'_> {
    #[track_caller]
    fn assert_layout_done(&self, child: &WidgetPod<impl Widget + ?Sized>, method_name: &str) {
        if self.get_child_state(child).needs_layout() {
            debug_panic!(
                "Error in {}: trying to call '{}' with child '{}' {} before computing its layout",
                self.widget_id(),
                method_name,
                self.get_child_dyn(child).short_type_name(),
                child.id(),
            );
        }
    }

    /// Computes the `child`'s preferred border-box size.
    ///
    /// The returned size will be finite, non-negative, and in device pixels.
    ///
    /// Container widgets usually call this method as part of their [`layout`] logic, but
    /// ultimately they can disregard the result and pass a different size to [`run_layout`].
    /// Read [`layout`] docs for more details about that process.
    ///
    /// `auto_size` specifies the fallback behavior if the child has any dimension as [`Dim::Auto`].
    /// Most widgets should use [`SizeDef::fit`] to ask the child to fit inside the
    /// available space. However sometimes a different fallback makes more sense, e.g.
    /// `Grid` uses [`SizeDef::fixed`] to fall back to the exact allocated child area size.
    ///
    /// `context_size` is the size, in device pixels, that is used to resolve relative sizes.
    /// For example [`Ratio(0.5)`] will result in half the context size.
    /// This is usually the container widget's content-box size, i.e. excluding borders and padding.
    /// Examples of exceptions include `Grid` which will provide the child's area size,
    /// i.e. the union of cell sizes that the child occupies, and `Portal` which will provide
    /// its viewport size.
    ///
    /// [`layout`]: Widget::layout
    /// [`Ratio(0.5)`]: crate::layout::Dim::Ratio
    /// [`Dim::Auto`]: crate::layout::Dim::Auto
    /// [`run_layout`]: Self::run_layout
    pub fn compute_size(
        &mut self,
        child: &mut WidgetPod<impl Widget + ?Sized>,
        auto_size: SizeDef,
        context_size: LayoutSize,
    ) -> Size {
        let id = child.id();
        let node = self.children.item_mut(id).unwrap();
        resolve_size(
            self.global_state,
            self.default_properties,
            node,
            auto_size,
            context_size,
        )
    }

    /// Lays out the `child` widget with a chosen border-box `size`.
    ///
    /// Container widgets must call this on every child as part of their [`layout`] method.
    ///
    /// The container widget should usually call [`compute_size`] on the `child`
    /// to get its preferred border-box `size`.
    /// However, ultimately the parent is in control and can choose any `size` for the child.
    ///
    /// If the chosen border-box `size` is smaller than what is required to fit the child's
    /// borders and padding, then the `size` will be expanded to meet those constraints.
    ///
    /// The provided `size` must be finite, non-negative, and in device pixels.
    /// Non-finite or negative size will fall back to zero with a logged warning.
    ///
    /// # Panics
    ///
    /// Panics if the provided `size` is non-finite or negative and debug assertions are enabled.
    ///
    /// [`layout`]: Widget::layout
    /// [`compute_size`]: Self::compute_size
    pub fn run_layout(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>, chosen_size: Size) {
        let id = child.id();
        let node = self.children.item_mut(id).unwrap();

        run_layout_on(
            self.global_state,
            self.default_properties,
            node,
            chosen_size,
        );

        let state_mut = &mut self.children.item_mut(id).unwrap().item.state;
        self.widget_state.merge_up(state_mut);
    }

    /// Sets the position of the `child` widget, in this widget's content-box coordinate space.
    ///
    /// Container widgets must call this method with each non-stashed child in their
    /// [`layout`] method, after calling `ctx.run_layout(child, size)`.
    ///
    /// # Panics
    ///
    /// This method will panic if [`LayoutCtx::run_layout`] has not been called yet for the child.
    ///
    /// [`layout`]: Widget::layout
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

        // Convert child's origin from this widget's content-box space to border-box space.
        let translation = self.widget_state.border_box_translation();
        let child_origin = origin + translation;
        let child_state = self.get_child_state_mut(child);

        place_widget(child_state, child_origin);
    }

    /// Sets explicit paint [`Insets`] for this widget.
    ///
    /// The argument is an [`Insets`] struct that indicates where your widget will overpaint,
    /// relative to its layout content-box, as defined by the `size` given to the widget's
    /// [`layout`] method.
    ///
    /// You are only required to notify of painting that actually overflows the layout border-box.
    /// The insets will still be relative to the content-box, it's just that Masonry doesn't
    /// really need to be notified if you're just painting over your padding or borders.
    ///
    /// You are only required to notify of painting done directly by this widget.
    /// Child widget overdraw needs to be reported by those child widgets themselves.
    ///
    /// [`layout`]: Widget::layout
    pub fn set_paint_insets(&mut self, insets: impl Into<Insets>) {
        let insets = insets.into();
        let insets = Insets::new(
            insets.x0 - self.widget_state.border_box_insets.x0,
            insets.y0 - self.widget_state.border_box_insets.y0,
            insets.x1 - self.widget_state.border_box_insets.x1,
            insets.y1 - self.widget_state.border_box_insets.y1,
        );
        self.widget_state.paint_insets = insets.nonnegative();
    }

    /// Sets an explicit baseline position for this widget.
    ///
    /// The baseline position is used to align widgets that contain text,
    /// such as buttons, labels, and other controls. It may also be used
    /// by other widgets that are opinionated about how they are aligned
    /// relative to neighbouring text, such as switches or checkboxes.
    ///
    /// The provided value must be the distance from the *bottom* of this
    /// widget's content-box to its baseline.
    pub fn set_baseline_offset(&mut self, baseline: f64) {
        self.widget_state.layout_baseline_offset =
            baseline + self.widget_state.border_box_insets.y1;
    }

    /// Clears an explicitly set baseline position for this widget.
    ///
    /// This results in the effective baseline being the bottom edge of this widget's border-box.
    pub fn clear_baseline_offset(&mut self) {
        self.widget_state.layout_baseline_offset = 0.;
    }

    /// Returns the insets for converting between content-box and border-box rects.
    ///
    /// Add these insets to the content-box to get the border-box,
    /// and subtract these insets from the border-box to get the content-box.
    pub fn border_box_insets(&mut self) -> Insets {
        self.widget_state.border_box_insets
    }

    /// Returns whether this widget needs to call [`LayoutCtx::run_layout`].
    pub fn needs_layout(&self) -> bool {
        self.widget_state.needs_layout()
    }

    /// Returns whether a child of this widget needs to call [`LayoutCtx::run_layout`].
    pub fn child_needs_layout(&self, child: &WidgetPod<impl Widget + ?Sized>) -> bool {
        self.get_child_state(child).needs_layout()
    }

    /// The distance from the bottom of the child widget's layout border-box to its baseline.
    ///
    /// # Panics
    ///
    /// This method will panic if [`LayoutCtx::run_layout`] has not been called yet for
    /// the child.
    #[track_caller]
    pub fn child_baseline_offset(&self, child: &WidgetPod<impl Widget + ?Sized>) -> f64 {
        self.assert_layout_done(child, "child_baseline_offset");
        self.get_child_state(child).layout_baseline_offset
    }

    /// Returns the given child's layout border-box size.
    ///
    /// # Panics
    ///
    /// This method will panic if [`LayoutCtx::run_layout`] has not been called yet for
    /// the child.
    #[track_caller]
    pub fn child_size(&self, child: &WidgetPod<impl Widget + ?Sized>) -> Size {
        self.assert_layout_done(child, "child_size");
        self.get_child_state(child).layout_border_box_size
    }

    /// Sets the widget's clip path in the widget's content-box coordinate space.
    ///
    /// A widget's clip path will have two effects:
    /// - It serves as a mask for painting operations of this widget and its children.
    ///   Note that while all painting done by children will be clipped by this path,
    ///   only the painting done in [`paint`] by this widget itself will be clipped.
    ///   The remaining painting done in [`pre_paint`] and [`post_paint`] will not be clipped.
    /// - Pointer events must be inside this path to reach the widget's children.
    ///
    /// [`paint`]: Widget::paint
    /// [`pre_paint`]: Widget::pre_paint
    /// [`post_paint`]: Widget::post_paint
    pub fn set_clip_path(&mut self, path: Rect) {
        // Translate the clip path to the widget's border-box coordinate space.
        let path = path + self.widget_state.border_box_translation();
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

    /// Removes the widget's clip path.
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
}

impl ComposeCtx<'_> {
    // TODO - Remove?
    /// Returns whether [`Widget::compose`] will be called on this widget.
    pub fn needs_compose(&self) -> bool {
        self.widget_state.needs_compose
    }

    /// Sets the scroll translation for the child widget.
    ///
    /// The translation is applied on top of the position from [`LayoutCtx::place_child`].
    ///
    /// The given translation may be quantized so the child's final position
    /// stays pixel-perfect.
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

        let translation = translation.round();

        let child = self.get_child_state_mut(child);
        if translation != child.scroll_translation {
            child.scroll_translation = translation;
            child.transform_changed = true;
        }
    }

    /// Sets the scroll translation for the child widget.
    ///
    /// The translation is applied on top of the position from [`LayoutCtx::place_child`].
    ///
    /// Unlike [`Self::set_child_scroll_translation`], doesn't perform pixel-snapping.
    /// This method should be used for intermediary scroll values during scroll animations.
    pub fn set_animated_child_scroll_translation(
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
                "Error in {}: trying to call 'set_animated_child_scroll_translation' with child '{}' {} with invalid translation {:?}",
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
// Methods on all context types except MeasureCtx and LayoutCtx
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
        /// Returns the aligned content-box size of this widget.
        pub fn content_box_size(&self) -> Size {
            let border_box_size = self.widget_state.border_box_size();
            Size::new(
                (border_box_size.width - self.widget_state.border_box_insets.x_value()).max(0.),
                (border_box_size.height - self.widget_state.border_box_insets.y_value()).max(0.),
            )
        }

        /// Returns the aligned border-box size of this widget.
        pub fn border_box_size(&self) -> Size {
            self.widget_state.border_box_size()
        }

        /// Returns the aligned paint-box size of this widget.
        pub fn paint_box_size(&self) -> Size {
            self.widget_state.paint_box().size()
        }

        /// Returns the aligned content-box rect of this widget
        /// in this widget's content-box coordinate space.
        pub fn content_box(&self) -> Rect {
            let border_box_size = self.widget_state.border_box_size();
            Rect::new(
                0.,
                0.,
                (border_box_size.width - self.widget_state.border_box_insets.x_value()).max(0.),
                (border_box_size.height - self.widget_state.border_box_insets.y_value()).max(0.),
            )
        }

        /// Returns the aligned border-box rect of this widget
        /// in this widget's content-box coordinate space.
        pub fn border_box(&self) -> Rect {
            let border_box_size = self.widget_state.border_box_size();
            let origin = Point::new(
                -self.widget_state.border_box_insets.x0,
                -self.widget_state.border_box_insets.y0,
            );
            Rect::from_origin_size(origin, border_box_size)
        }

        /// Returns the aligned paint-box rect of this widget
        /// in this widget's content-box coordinate space.
        ///
        /// Covers the area we expect to be invalidated when the widget is painted.
        pub fn paint_box(&self) -> Rect {
            let translation = self.widget_state.border_box_translation();
            self.widget_state.paint_box() - translation
        }

        /// Returns the widget's bounding-box rect in the window's coordinate space.
        ///
        /// It contains this widget and all of its descendents.
        ///
        /// This is the union of clipped effective paint-box rects, i.e. the union of
        /// globally transformed aligned border-box rects with paint insets applied.
        ///
        /// See [bounding box documentation] for more details.
        ///
        /// [bounding box documentation]: crate::doc::masonry_concepts#bounding-box
        pub fn bounding_box(&self) -> Rect {
            self.widget_state.bounding_box
        }

        /// Returns the baseline offset relative to the bottom of the widget's aligned content-box.
        pub fn baseline_offset(&self) -> f64 {
            let border_box_baseline = self.widget_state.baseline_offset();
            border_box_baseline - self.widget_state.border_box_insets.y1
        }

        /// The clip path of the widget, if any was set.
        ///
        /// The returned clip path will be in this widget's content-box coordinate space.
        ///
        /// For more information, see
        /// [`LayoutCtx::set_clip_path`](crate::core::LayoutCtx::set_clip_path).
        pub fn clip_path(&self) -> Option<Rect> {
            // Translate the clip path to the widget's content-box coordinate space.
            let translation = self.widget_state.border_box_translation();
            self.widget_state.clip_path.map(|path| path - translation)
        }

        /// Returns the [`Vec2`] for translating between this widget's
        /// content-box and border-box coordinate spaces.
        ///
        /// Add this [`Vec2`] to translate from content-box to border-box,
        /// and subtract this [`Vec2`] to translate from border-box to content-box.
        pub fn border_box_translation(&self) -> Vec2 {
            self.widget_state.border_box_translation()
        }

        /// Returns the widget's effective border-box origin in the window's coordinate space.
        pub fn window_origin(&self) -> Point {
            self.widget_state.border_box_window_origin()
        }

        /// Returns the global transform mapping this widget's content-box coordinate space
        /// to the window's coordinate space.
        ///
        /// Computed from all `transform`, `scroll_translation`, and `origin` values
        /// from this widget all the way up to the window.
        ///
        /// Multiply by this to convert from this widget's content-box coordinate space to the window's,
        /// or use the inverse of this transform to go from window's space to this widget's content-box.
        pub fn window_transform(&self) -> Affine {
            let translation = self.widget_state.border_box_translation();
            self.widget_state
                .window_transform
                .pre_translate(translation)
        }

        /// Converts the `point` from the window's coordinate space
        /// to this widget's content-box coordinate space.
        pub fn to_local(&self, point: Point) -> Point {
            let to_border_box = self.widget_state.window_transform.inverse();
            let to_content_box = -self.widget_state.border_box_translation();
            to_border_box.then_translate(to_content_box) * point
        }

        /// Converts the `point` from this widget's content-box coordinate space
        /// to the window's coordinate space.
        ///
        /// The returned point is relative to the window's content area; it excludes window chrome.
        pub fn to_window(&self, point: Point) -> Point {
            let translation = self.widget_state.border_box_translation();
            self.widget_state.window_transform * (point + translation)
        }
    }
);

impl_context_method!(AccessCtx<'_>, EventCtx<'_>, PaintCtx<'_>, {
    // TODO - Once Masonry uses physical coordinates, add this method everywhere.
    // See https://github.com/linebender/xilem/issues/1264
    /// Returns DPI scaling factor.
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

// --- MARK: GET STATUS

// Methods on all context types
// Access status information (hovered/pointer captured/disabled/etc).
impl_context_method!(
    MutateCtx<'_>,
    QueryCtx<'_>,
    EventCtx<'_>,
    UpdateCtx<'_>,
    MeasureCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    PaintCtx<'_>,
    AccessCtx<'_>,
    {
        /// The "hovered" status of a widget.
        ///
        /// A widget is "hovered" when a pointer is hovering over its border-box.
        /// Widgets will often change their appearance as a visual indication that they
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

        /// The widget, if any, which has [pointer capture].
        ///
        /// The pointer will usually be the mouse. In future versions, this
        /// function will take a pointer id as input to test a specific pointer.
        ///
        /// [pointer capture]: crate::doc::masonry_concepts#pointer-capture
        pub fn pointer_capture_target_id(&self) -> Option<WidgetId> {
            self.global_state.pointer_capture_target
        }

        /// The widget, if any, which has [text focus].
        ///
        /// The focused widget is the one that receives keyboard events.
        ///
        /// [text focus]: crate::doc::masonry_concepts#text-focus
        pub fn focus_target_id(&self) -> Option<WidgetId> {
            self.global_state.focused_widget
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

        /// Whether this widget is [disabled].
        ///
        /// Returns `true` if this widget or any of its ancestors is explicitly disabled.
        /// To make this widget explicitly disabled use [`set_disabled`].
        ///
        /// [disabled]: crate::doc::masonry_concepts#disabled
        /// [`set_disabled`]: EventCtx::set_disabled
        pub fn is_disabled(&self) -> bool {
            self.widget_state.is_disabled
        }

        /// Whether this widget is [stashed].
        ///
        /// [stashed]: crate::doc::masonry_concepts#stashed
        pub fn is_stashed(&self) -> bool {
            self.widget_state.is_stashed
        }
    }
);

// --- MARK: UPDATE FLAGS
impl_context_method!(MutateCtx<'_>, EventCtx<'_>, UpdateCtx<'_>, RawCtx<'_>, {
    /// Requests a [`paint`](crate::core::Widget::paint) and an
    /// [`accessibility`](crate::core::Widget::accessibility) pass.
    pub fn request_render(&mut self) {
        trace!("request_render");
        self.widget_state.request_pre_paint = true;
        self.widget_state.request_paint = true;
        self.widget_state.request_post_paint = true;
        self.widget_state.needs_paint = true;
        self.widget_state.needs_accessibility = true;
        self.widget_state.request_accessibility = true;
    }

    /// Requests a paint pass for the [`pre_paint`](crate::core::Widget::pre_paint) method.
    pub fn request_pre_paint(&mut self) {
        trace!("request_pre_paint");
        self.widget_state.request_pre_paint = true;
        self.widget_state.needs_paint = true;
    }

    /// Requests a paint pass, specifically for the [`paint`] method.
    ///
    /// Unlike [`request_render`], this does not request an [`accessibility`] pass
    /// or a call to [`pre_paint`] or [`post_paint`].
    ///
    /// Use `request_render` unless you're sure those aren't needed.
    ///
    /// [`paint`]: crate::core::Widget::paint
    /// [`request_render`]: Self::request_render
    /// [`accessibility`]: crate::core::Widget::accessibility
    /// [`pre_paint`]: crate::core::Widget::post_paint
    /// [`post_paint`]: crate::core::Widget::post_paint
    pub fn request_paint_only(&mut self) {
        trace!("request_paint_only");
        self.widget_state.request_paint = true;
        self.widget_state.needs_paint = true;
    }

    /// Requests a paint pass for the [`post_paint`](crate::core::Widget::post_paint) method.
    pub fn request_post_paint(&mut self) {
        trace!("request_post_paint");
        self.widget_state.request_post_paint = true;
        self.widget_state.needs_paint = true;
    }

    /// Requests an [`accessibility`](crate::core::Widget::accessibility) pass.
    ///
    /// This doesn't request a [`paint`](crate::core::Widget::paint) pass.
    /// If you want to request both an accessibility pass and a paint pass,
    /// use [`request_render`](Self::request_render).
    pub fn request_accessibility_update(&mut self) {
        trace!("request_accessibility_update");
        self.widget_state.needs_accessibility = true;
        self.widget_state.request_accessibility = true;
    }

    /// Requests a [`layout`] pass.
    ///
    /// Call this method if the widget has changed in a way that requires a layout pass.
    ///
    /// [`layout`]: crate::core::Widget::layout
    pub fn request_layout(&mut self) {
        trace!("request_layout");
        self.widget_state.request_layout = true;
        self.widget_state.set_needs_layout(true);
    }

    // TODO - Document better
    /// Requests a [`compose`] pass.
    ///
    /// The compose pass is often cheaper than the layout pass,
    /// because it can only transform individual widgets' position.
    ///
    /// [`compose`]: crate::core::Widget::compose
    pub fn request_compose(&mut self) {
        trace!("request_compose");
        self.widget_state.needs_compose = true;
        self.widget_state.request_compose = true;
    }

    /// Requests an animation frame.
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

    /// Indicates that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    ///
    /// This method will also call [`request_layout`](Self::request_layout).
    pub fn children_changed(&mut self) {
        trace!("children_changed");
        self.widget_state.children_changed = true;
        self.widget_state.needs_update_focusable = true;
        self.request_layout();
    }

    /// Indicates that a child is about to be removed from the tree.
    ///
    /// Container widgets should avoid dropping `WidgetPod`s. Instead, they should
    /// pass them to this method.
    ///
    /// This method will also call [`children_changed`](Self::children_changed).
    pub fn remove_child(&mut self, child: WidgetPod<impl Widget + ?Sized>) {
        fn remove_node(
            global_state: &mut RenderRootState,
            parent_state: &mut WidgetState,
            node: ArenaMut<'_, WidgetArenaNode>,
        ) {
            let mut children = node.children;
            let widget = &mut *node.item.widget;
            let state = &mut node.item.state;

            // TODO - Send event to widget

            let parent_name = widget.short_type_name();
            let parent_id = state.id;
            for child_id in widget.children_ids() {
                let Some(node) = children.item_mut(child_id) else {
                    panic!(
                        "Error in '{parent_name}' {parent_id}: cannot find child {child_id} returned by children_ids()"
                    );
                };

                remove_node(global_state, state, node);
            }

            // If we remove the focus anchor, its parent becomes the anchor.
            if global_state.focus_anchor == Some(state.id) {
                global_state.focus_anchor = Some(parent_state.id);
            }

            global_state.scene_cache.remove(&state.id);
        }

        let id = child.id();
        let node = self
            .children
            .item_mut(id)
            .expect("remove_child: child not found");
        remove_node(self.global_state, self.widget_state, node);

        let _ = self.children.remove(id).unwrap();

        self.children_changed();
    }

    /// Sets the disabled state for this widget.
    ///
    /// Setting this to `false` does not mean a widget is not still disabled; for instance it may
    /// still be disabled by an ancestor. See [`is_disabled`] for more information.
    ///
    /// [`is_disabled`]: EventCtx::is_disabled
    pub fn set_disabled(&mut self, disabled: bool) {
        self.widget_state.needs_update_disabled = true;
        self.widget_state.is_explicitly_disabled = disabled;
    }

    /// Sets the local transform for this widget.
    ///
    /// This maps this widget's border-box coordinate space
    /// to the parent's border-box coordinate space.
    ///
    /// It behaves similarly as CSS transforms.
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
    MeasureCtx<'_>,
    LayoutCtx<'_>,
    ComposeCtx<'_>,
    RawCtx<'_>,
    {
        // TODO - Remove from MeasureCtx/LayoutCtx/ComposeCtx
        /// Marks child widget as stashed.
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
                child_state.is_explicitly_stashed = stashed;
                child_state.needs_update_stashed = true;
                self.widget_state.needs_update_stashed = true;
            }
        }

        // TODO - Remove from MutateCtx?
        /// Queues a callback that will be called with a [`WidgetMut`] for this widget.
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

        /// Queues a callback that will be called with a [`WidgetMut`] for the given child widget.
        ///
        /// The callbacks will be run in the order they were submitted during the mutate pass.
        pub fn mutate_child_later<W: Widget + FromDynWidget + ?Sized>(
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

        /// Queues a callback that will be called with a [`WidgetMut`] for the widget with the given id.
        ///
        /// The callbacks will be run in the order they were submitted during the mutate pass.
        pub fn mutate_later(
            &mut self,
            target: WidgetId,
            f: impl FnOnce(WidgetMut<'_, dyn Widget>) + Send + 'static,
        ) {
            let callback = MutateCallback {
                id: target,
                callback: Box::new(|mut widget_mut| f(widget_mut.downcast())),
            };
            self.global_state.mutate_callbacks.push(callback);
        }

        /// Returns direct reference to the stored child, and a context handle for that child.
        pub fn get_raw<Child: Widget + FromDynWidget + ?Sized>(
            &mut self,
            child: &mut WidgetPod<Child>,
        ) -> (&Child, RawCtx<'_>) {
            let node_mut = self
                .children
                .item_mut(child.id())
                .expect("get_mut: child not found");
            let child_ctx = RawCtx {
                global_state: self.global_state,
                parent_widget_state: self.widget_state,
                widget_state: &mut node_mut.item.state,
                children: node_mut.children,
                default_properties: self.default_properties,
            };

            let widget = Child::from_dyn(&*node_mut.item.widget).unwrap();

            (widget, child_ctx)
        }

        /// Returns direct mutable reference to the stored child, and a context handle for that child.
        ///
        /// This context lets you set pass flags for the child widget, which may be tricky.
        /// In general, you should avoid setting the flags of a pass that runs before the
        /// pass you're currently in.
        /// Not doing so might lead to performance cliffs and, hypothetically, panics.
        ///
        /// This method is an escape hatch for cases where a parent widget completely
        /// controls their child, but needs it to be a separate widget for user interaction to
        /// behave as expected.
        /// As such, the child widget must opt-in using the `AllowRawMut` trait.
        ///
        /// See [pass documentation](crate::doc::pass_system) for the pass order.
        pub fn get_raw_mut<Child: Widget + FromDynWidget + AllowRawMut + ?Sized>(
            &mut self,
            child: &mut WidgetPod<Child>,
        ) -> (&mut Child, RawCtx<'_>) {
            let node_mut = self
                .children
                .item_mut(child.id())
                .expect("get_mut: child not found");
            let child_ctx = RawCtx {
                global_state: self.global_state,
                parent_widget_state: self.widget_state,
                widget_state: &mut node_mut.item.state,
                children: node_mut.children,
                default_properties: self.default_properties,
            };

            let widget = Child::from_dyn_mut(&mut *node_mut.item.widget).unwrap();

            (widget, child_ctx)
        }

        /// Submits an action, which indicates that this widget requires something be handled
        /// by the application, such as user input.
        ///
        /// The `Action` type parameter should always be the `Self::Action` associated type
        /// of the widget you're calling this method from.
        /// Masonry will validate this, and this method may panic if this isn't the case.
        ///
        /// For further details see [`ErasedAction`].
        pub fn submit_action<Action: AnyDebug + Send>(&mut self, action: impl Into<Action>) {
            trace!("submit_action");
            let action = action.into();
            if action.type_id() != self.widget_state.action_type {
                #[cfg(debug_assertions)]
                let expected_type = self.widget_state.action_type_name;
                #[cfg(not(debug_assertions))]
                let expected_type = "<Self as Widget>::Action";

                debug_panic!(
                    "Trying to emit action of incorrect type `{}`. Expected type is `{}`.",
                    action.type_name(),
                    expected_type,
                );
                return;
            }
            self.global_state.emit_signal(RenderRootSignal::Action(
                Box::new(action),
                self.widget_state.id,
            ));
        }

        /// Submits a type-erased action.
        ///
        /// Unlike [`Self::submit_action`], this method lets you submit an action with an
        /// arbitrary type, which may not match `Self::Action`.
        /// This may act as an escape hatch in some situations.
        ///
        /// For further details see [`ErasedAction`].
        pub fn submit_untyped_action(&mut self, action: ErasedAction) {
            trace!("submit_untyped_action");
            self.global_state
                .emit_signal(RenderRootSignal::Action(action, self.widget_state.id));
        }

        /// Sets the IME cursor area in the widget's content-box coordinate space.
        ///
        /// When this widget is [focused] and [accepts text input], the reported IME area is sent
        /// to the platform. The area can be used by the platform to, for example, place a
        /// candidate box near that area, while ensuring the area is not obscured.
        ///
        /// If no IME area is set, then Masonry will use the widget's aligned border-box rect.
        ///
        /// [focused]: EventCtx::request_focus
        /// [accepts text input]: Widget::accepts_text_input
        pub fn set_ime_area(&mut self, ime_area: Rect) {
            let translation = self.widget_state.border_box_translation();
            self.widget_state.ime_area = Some(ime_area + translation);
        }

        /// Removes the IME cursor area.
        ///
        /// See [`LayoutCtx::set_ime_area`](LayoutCtx::set_ime_area) for more details.
        pub fn clear_ime_area(&mut self) {
            self.widget_state.ime_area = None;
        }

        /// Sets the contents of the platform clipboard.
        ///
        /// For example, text widgets should call this for "cut" and "copy" user interactions.
        /// Note that we currently don't support the "Primary" selection buffer on X11/Wayland.
        pub fn set_clipboard(&mut self, contents: String) {
            trace!("set_clipboard");
            self.global_state
                .emit_signal(RenderRootSignal::ClipboardStore(contents));
        }

        /// Starts a window drag.
        ///
        /// Moves the window with the left mouse button until the button is released.
        pub fn drag_window(&mut self) {
            trace!("drag_window");
            self.global_state.emit_signal(RenderRootSignal::DragWindow);
        }

        /// Starts a window resize.
        ///
        /// Resizes the window with the left mouse button until the button is released.
        pub fn drag_resize_window(&mut self, direction: ResizeDirection) {
            trace!("drag_resize_window");
            self.global_state
                .emit_signal(RenderRootSignal::DragResizeWindow(direction));
        }

        /// Toggles the maximized state of the window.
        pub fn toggle_maximized(&mut self) {
            trace!("toggle_maximized");
            self.global_state
                .emit_signal(RenderRootSignal::ToggleMaximized);
        }

        /// Minimizes the window.
        pub fn minimize(&mut self) {
            trace!("minimize");
            self.global_state.emit_signal(RenderRootSignal::Minimize);
        }

        /// Exits the application.
        pub fn exit(&mut self) {
            trace!("exit");
            self.global_state.emit_signal(RenderRootSignal::Exit);
        }

        /// Shows the window menu at a specified position.
        pub fn show_window_menu(&mut self, position: LogicalPosition<f64>) {
            trace!("show_window_menu");
            self.global_state
                .emit_signal(RenderRootSignal::ShowWindowMenu(position));
        }

        /// Creates a new [layer] at a specified `position`.
        ///
        /// The given `position` must be in the window's coordinate space.
        ///
        /// # Panics
        ///
        /// If [`W::as_layer()`](Widget::as_layer) returns `None`.
        ///
        /// [layer]: crate::doc::masonry_concepts#layers
        pub fn create_layer<W: Widget + ?Sized>(
            &mut self,
            layer_type: LayerType,
            mut fallback_widget: NewWidget<W>,
            position: Point,
        ) {
            trace!("create_layer");

            if fallback_widget.widget.as_layer().is_none() {
                debug_panic!(
                    "cannot create layer of type {} - `Widget::as_layer()` returned None",
                    fallback_widget.widget.short_type_name()
                );
                return;
            }

            self.global_state.emit_signal(RenderRootSignal::NewLayer(
                layer_type,
                fallback_widget.erased(),
                position,
            ));
        }

        /// Removes the layer with the specified widget as root.
        pub fn remove_layer(&mut self, root_widget_id: WidgetId) {
            trace!("remove_layer");
            self.global_state
                .emit_signal(RenderRootSignal::RemoveLayer(root_widget_id));
        }

        /// Repositions the layer with the specified widget as root.
        ///
        /// The given `position` must be in the window's coordinate space.
        pub fn reposition_layer(&mut self, root_widget_id: WidgetId, position: Point) {
            trace!("reposition_layer");
            self.global_state
                .emit_signal(RenderRootSignal::RepositionLayer(root_widget_id, position));
        }
    }
);

impl RegisterCtx<'_> {
    /// Registers a child widget.
    ///
    /// Container widgets should call this on all their children in
    /// their implementation of [`Widget::register_children`].
    pub fn register_child(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>) {
        let Some(NewWidget {
            widget,
            id,
            options,
            properties,
            tag,
            action_type,
            #[cfg(debug_assertions)]
            action_type_name,
        }) = child.take_inner()
        else {
            return;
        };

        #[cfg(debug_assertions)]
        {
            self.registered_ids.push(id);
        }

        let state = WidgetState::new(
            id,
            widget.short_type_name(),
            options,
            action_type,
            #[cfg(debug_assertions)]
            action_type_name,
        );

        if let Some(tag) = tag {
            let entry = self.global_state.widget_tags.entry(tag);

            let Entry::Vacant(vacant_entry) = entry else {
                debug_panic!("Tag '{tag}' already exists in the widget tree");
                return;
            };

            vacant_entry.insert(id);
        }

        let node = WidgetArenaNode {
            widget: widget.as_box_dyn(),
            state,
            properties: properties.map,
            changed_properties: TypeSet::default(),
        };
        self.children.insert(id, node);
    }
}

impl Drop for RawCtx<'_> {
    fn drop(&mut self) {
        self.parent_widget_state.merge_up(self.widget_state);
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
        self.global_state.debug_paint
    }

    /// A color used for debug painting in this widget.
    ///
    /// This is normally used to paint additional debugging information
    /// when debug paint is enabled, see [`Self::debug_paint_enabled`].
    pub fn debug_color(&self) -> Color {
        get_debug_color(self.widget_id().to_raw())
    }
}
