// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Core types and mechanisms for the widget hierarchy.
//!
//! //! Note: the organization of this code roughly follows the existing Druid
//! widget system, particularly its core.rs.

use bitflags::bitflags;
use vello::kurbo::{Affine, Point, Rect, Size};
use vello::{SceneBuilder, SceneFragment};

use super::widget::{AnyWidget, Widget};
use crate::Axis;
use crate::{id::Id, Bloom};

use super::{
    contexts::LifeCycleCx, AccessCx, BoxConstraints, CxState, Event, EventCx, LayoutCx, LifeCycle,
    PaintCx, UpdateCx,
};

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub(crate) struct PodFlags: u32 {
        // These values are set to the values of their pendants in ChangeFlags to allow transmuting
        // between the two types.
        const REQUEST_UPDATE = ChangeFlags::UPDATE.bits() as _;
        const REQUEST_LAYOUT = ChangeFlags::LAYOUT.bits() as _;
        const REQUEST_ACCESSIBILITY = ChangeFlags::ACCESSIBILITY.bits() as _;
        const REQUEST_PAINT = ChangeFlags::PAINT.bits() as _;
        const TREE_CHANGED = ChangeFlags::TREE.bits() as _;
        const DESCENDANT_REQUESTED_ACCESSIBILITY = ChangeFlags::DESCENDANT_REQUESTED_ACCESSIBILITY.bits() as _;

        // Everything else uses bitmasks greater than the max value of ChangeFlags: mask >= 0x100
        const VIEW_CONTEXT_CHANGED = 0x100;

        const IS_HOT = 0x200;
        const IS_ACTIVE = 0x400;
        const HAS_ACTIVE = 0x800;

        const NEEDS_SET_ORIGIN = 0x1000;

        const UPWARD_FLAGS = Self::REQUEST_UPDATE.bits()
            | Self::REQUEST_LAYOUT.bits()
            | Self::REQUEST_PAINT.bits()
            | Self::HAS_ACTIVE.bits()
            | Self::DESCENDANT_REQUESTED_ACCESSIBILITY.bits()
            | Self::TREE_CHANGED.bits()
            | Self::VIEW_CONTEXT_CHANGED.bits();
        const INIT_FLAGS = Self::REQUEST_UPDATE.bits()
            | Self::REQUEST_LAYOUT.bits()
            | Self::REQUEST_ACCESSIBILITY.bits()
            | Self::DESCENDANT_REQUESTED_ACCESSIBILITY.bits()
            | Self::REQUEST_PAINT.bits()
            | Self::TREE_CHANGED.bits();
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    #[must_use]
    pub struct ChangeFlags: u8 {
        const UPDATE = 1;
        const LAYOUT = 2;
        const ACCESSIBILITY = 4;
        const PAINT = 8;
        const TREE = 0x10;
        const DESCENDANT_REQUESTED_ACCESSIBILITY = 0x20;
    }
}

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `Pod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
///
/// `Pod` will translate internal Xilem events to regular events,
/// synthesize additional events of interest, and stop propagation when it makes sense.
pub struct Pod {
    pub(crate) state: WidgetState,
    pub(crate) widget: Box<dyn AnyWidget>,
    pub(crate) fragment: SceneFragment,
}

#[derive(Debug)]
pub(crate) struct WidgetState {
    pub(crate) id: Id,
    pub(crate) flags: PodFlags,
    /// The origin of the child in the parent's coordinate space.
    pub(crate) origin: Point,
    /// The origin of the parent in the window coordinate space.
    pub(crate) parent_window_origin: Point,
    /// The size of the widget.
    pub(crate) size: Size,
    /// A bloom filter containing this widgets is and the ones of its children.
    // TODO: decide the final solution for this. This is probably going to be a global structure
    //       tracking parent child relations in the tree:
    //           parents: HashMap<Id, Id>,
    //           children: HashMap<Id, Vec<Id>>,
    pub(crate) sub_tree: Bloom<Id>,
}

impl PodFlags {
    /// Flags to be propagated upwards.
    pub(crate) fn upwards(self) -> Self {
        let mut result = self & PodFlags::UPWARD_FLAGS;
        if self.contains(PodFlags::REQUEST_ACCESSIBILITY) {
            result |= PodFlags::DESCENDANT_REQUESTED_ACCESSIBILITY;
        }
        result
    }
}

impl ChangeFlags {
    pub(crate) fn upwards(self) -> Self {
        // Note: this assumes PodFlags are a superset of ChangeFlags. This might
        // not always be the case, for example on "structure changed."
        let pod_flags = PodFlags::from_bits_truncate(self.bits() as _);
        ChangeFlags::from_bits_truncate(pod_flags.upwards().bits() as _)
    }

    // Change flags representing change of tree structure.
    pub fn tree_structure() -> Self {
        ChangeFlags::TREE
    }
}

impl WidgetState {
    pub(crate) fn new() -> Self {
        let id = Id::next();
        WidgetState {
            id,
            flags: PodFlags::INIT_FLAGS,
            origin: Default::default(),
            parent_window_origin: Default::default(),
            size: Default::default(),
            sub_tree: Default::default(),
        }
    }

    fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.flags |= child_state.flags.upwards();
        self.sub_tree = self.sub_tree.union(child_state.sub_tree);
    }

    fn request(&mut self, flags: PodFlags) {
        self.flags |= flags
    }

    pub(crate) fn window_origin(&self) -> Point {
        self.parent_window_origin + self.origin.to_vec2()
    }
}

impl Pod {
    /// Create a new pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `Pod`
    /// so it can participate in layout and event flow.
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self::new_from_box(Box::new(widget))
    }

    /// Create a new pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `Pod`
    /// so it can participate in layout and event flow.
    pub fn new_from_box(widget: Box<dyn AnyWidget>) -> Self {
        Pod {
            state: WidgetState::new(),
            fragment: SceneFragment::default(),
            widget,
        }
    }

    /// Returns the wrapped widget.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        (*self.widget).as_any().downcast_ref()
    }

    /// Returns the wrapped widget.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (*self.widget).as_any_mut().downcast_mut()
    }

    /// Sets the requested flags on this pod and returns the ChangeFlags the owner of this Pod should set.
    pub fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags {
        self.state
            .request(PodFlags::from_bits_truncate(flags.bits() as _));
        flags.upwards()
    }

    /// Propagate a platform event. As in Druid, a great deal of the event
    /// dispatching logic is in this function.
    ///
    /// This method calls [event](crate::widget::Widget::event) on the wrapped Widget if this event
    /// is relevant to this widget.
    pub fn event(&mut self, cx: &mut EventCx, event: &Event) {
        if cx.is_handled {
            return;
        }
        let mut modified_event = None;
        let had_active = self.state.flags.contains(PodFlags::HAS_ACTIVE);
        let recurse = match event {
            Event::MouseDown(mouse_event) => {
                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseDown(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseUp(mouse_event) => {
                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseUp(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseMove(mouse_event) => {
                let hot_changed = Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) || hot_changed {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseMove(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseWheel(mouse_event) => {
                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    Some(mouse_event.pos),
                );
                if had_active || self.state.flags.contains(PodFlags::IS_HOT) {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= self.state.origin.to_vec2();
                    modified_event = Some(Event::MouseWheel(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseLeft() => {
                let hot_changed =
                    Pod::set_hot_state(&mut self.widget, &mut self.state, cx.cx_state, None);
                had_active || hot_changed
            }
            Event::TargetedAccessibilityAction(action) => {
                // println!("TODO: {:?}", action);
                self.state
                    .sub_tree
                    .may_contain(&Id::try_from_accesskit(action.target).unwrap())
            }
        };
        if recurse {
            // This clears the has_active state. Pod needs to clear this state since merge up can
            // only set flags.
            // This needs to happen before the `event` call, as that will also set our `HAS_ACTIVE`
            // flag if any of our children were active
            self.state.flags.set(
                PodFlags::HAS_ACTIVE,
                self.state.flags.contains(PodFlags::IS_ACTIVE),
            );
            let mut inner_cx = EventCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
                is_handled: false,
            };
            self.widget
                .event(&mut inner_cx, modified_event.as_ref().unwrap_or(event));
            cx.is_handled |= inner_cx.is_handled;

            cx.widget_state.merge_up(&mut self.state);
        }
    }

    /// Propagate a lifecycle event.
    ///
    /// This method calls [lifecycle](crate::widget::Widget::lifecycle) on the wrapped Widget if
    /// the lifecycle event is relevant to this widget.
    pub fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        let mut modified_event = None;
        let recurse = match event {
            LifeCycle::HotChanged(_) => false,
            LifeCycle::ViewContextChanged(view) => {
                self.state.parent_window_origin = view.window_origin;

                Pod::set_hot_state(
                    &mut self.widget,
                    &mut self.state,
                    cx.cx_state,
                    view.mouse_position,
                );
                modified_event = Some(LifeCycle::ViewContextChanged(
                    view.translate_to(self.state.origin),
                ));
                self.state.flags.remove(PodFlags::VIEW_CONTEXT_CHANGED);
                true
            }
            LifeCycle::TreeUpdate => {
                if self.state.flags.contains(PodFlags::TREE_CHANGED) {
                    self.state.sub_tree.clear();
                    self.state.sub_tree.add(&self.state.id);
                    self.state.flags.remove(PodFlags::TREE_CHANGED);
                    true
                } else {
                    false
                }
            }
        };
        let mut child_cx = LifeCycleCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        if recurse {
            self.widget
                .lifecycle(&mut child_cx, modified_event.as_ref().unwrap_or(event));
            cx.widget_state.merge_up(&mut self.state);
        }
    }

    /// Propagate an update.
    ///
    /// This method calls [update](crate::widget::Widget::update) on the wrapped Widget if update
    /// was request by this widget or any of its children.
    pub fn update(&mut self, cx: &mut UpdateCx) {
        if self.state.flags.contains(PodFlags::REQUEST_UPDATE) {
            let mut child_cx = UpdateCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            self.widget.update(&mut child_cx);
            self.state.flags.remove(PodFlags::REQUEST_UPDATE);
            cx.widget_state.merge_up(&mut self.state);
        }
    }

    /// Propagate a layout request.
    ///
    /// This method calls [layout](crate::widget::Widget::layout) on the wrapped Widget. The container
    /// widget is responsible for calling only the children which need a call to layout. These include
    /// any Pod which has [layout_requested](Pod::layout_requested) set.
    pub fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let mut child_cx = LayoutCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        let new_size = self.widget.layout(&mut child_cx, bc);
        //println!("layout size = {:?}", new_size);
        self.state.size = new_size;
        // Note: here we're always doing requests for downstream processing, but if we
        // make layout more incremental, we'll probably want to do this only if there
        // is an actual layout change.
        self.state
            .flags
            .insert(PodFlags::NEEDS_SET_ORIGIN | PodFlags::REQUEST_ACCESSIBILITY);
        self.state.flags.remove(PodFlags::REQUEST_LAYOUT);
        cx.widget_state.merge_up(&mut self.state);
        self.state.size
    }

    pub fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        cx: &mut LayoutCx,
        bc: &BoxConstraints,
    ) -> f64 {
        let mut child_cx = LayoutCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        self.widget.compute_max_intrinsic(axis, &mut child_cx, bc)
    }

    ///
    pub fn accessibility(&mut self, cx: &mut AccessCx) {
        if self.state.flags.intersects(
            PodFlags::REQUEST_ACCESSIBILITY | PodFlags::DESCENDANT_REQUESTED_ACCESSIBILITY,
        ) {
            let mut child_cx = AccessCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
                update: cx.update,
                node_classes: cx.node_classes,
            };
            self.widget.accessibility(&mut child_cx);
            self.state.flags.remove(
                PodFlags::REQUEST_ACCESSIBILITY | PodFlags::DESCENDANT_REQUESTED_ACCESSIBILITY,
            );
        }
    }

    pub fn paint_raw(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        let mut inner_cx = PaintCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        self.widget.paint(&mut inner_cx, builder);
    }

    pub(crate) fn paint_impl(&mut self, cx: &mut PaintCx) {
        let needs_paint = self.state.flags.contains(PodFlags::REQUEST_PAINT);
        self.state.flags.remove(PodFlags::REQUEST_PAINT);

        let mut inner_cx = PaintCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };

        if needs_paint {
            let mut builder = SceneBuilder::for_fragment(&mut self.fragment);
            self.widget.paint(&mut inner_cx, &mut builder);
        }
    }

    /// The default paint method.
    ///
    /// It paints the this widget if necessary and appends its SceneFragment to the provided
    /// `SceneBuilder`.
    pub fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        self.paint_impl(cx);
        let transform = Affine::translate(self.state.origin.to_vec2());
        builder.append(&self.fragment, Some(transform));
    }

    /// Renders the widget and returns the created `SceneFragment`.
    ///
    /// The caller of this method is responsible for translating the Fragment and appending it to
    /// its own SceneBuilder. This is useful for ClipBoxes and doing animations.
    ///
    /// For the default paint behaviour call [`paint`](Pod::paint).
    pub fn paint_custom(&mut self, cx: &mut PaintCx) -> &SceneFragment {
        self.paint_impl(cx);
        &self.fragment
    }

    /// Set the origin of this widget, in the parent's coordinate space.
    ///
    /// A container widget should call the [`Widget::layout`] method on its children in
    /// its own [`Widget::layout`] implementation, and then call `set_origin` to
    /// position those children.
    ///
    /// The changed origin won't be fully in effect until [`LifeCycle::ViewContextChanged`] has
    /// finished propagating. Specifically methods that depend on the widget's origin in relation
    /// to the window will return stale results during the period after calling `set_origin` but
    /// before [`LifeCycle::ViewContextChanged`] has finished propagating.
    ///
    /// The widget container can also call `set_origin` from other context, but calling `set_origin`
    /// after the widget received [`LifeCycle::ViewContextChanged`] and before the next event results
    /// in an inconsistent state of the widget tree.
    pub fn set_origin(&mut self, cx: &mut LayoutCx, origin: Point) {
        if origin != self.state.origin {
            self.state.origin = origin;
            // request paint is called on the parent instead of this widget, since this widget's
            // fragment does not change.
            cx.view_context_changed();
            cx.request_paint();

            self.state.flags.insert(PodFlags::VIEW_CONTEXT_CHANGED);
        }
    }

    // Return true if hot state has changed
    fn set_hot_state(
        widget: &mut dyn AnyWidget,
        widget_state: &mut WidgetState,
        cx_state: &mut CxState,
        mouse_pos: Option<Point>,
    ) -> bool {
        let rect = Rect::from_origin_size(widget_state.origin, widget_state.size);
        let had_hot = widget_state.flags.contains(PodFlags::IS_HOT);
        let is_hot = match mouse_pos {
            Some(pos) => rect.contains(pos),
            None => false,
        };
        widget_state.flags.set(PodFlags::IS_HOT, is_hot);
        if had_hot != is_hot {
            let hot_changed_event = LifeCycle::HotChanged(is_hot);
            let mut child_cx = LifeCycleCx {
                cx_state,
                widget_state,
            };
            widget.lifecycle(&mut child_cx, &hot_changed_event);
            return true;
        }
        false
    }

    /// Get the id of the widget in the pod.
    pub fn id(&self) -> Id {
        self.state.id
    }

    /// The "hot" (aka hover) status of a widget.
    ///
    /// A widget is "hot" when the mouse is hovered over it. Some Widgets (eg buttons)
    /// will change their appearance when hot as a visual indication that they
    /// will respond to mouse interaction.
    ///
    /// The hot status is automatically computed from the widget's layout rect. In a
    /// container hierarchy, all widgets with layout rects containing the mouse position
    /// have hot status. The hot status cannot be set manually.
    ///
    /// There is no special handling of the hot status for multi-pointer devices. (This is
    /// likely to change in the future as [pointer events are planed](https://xi.zulipchat.com/#narrow/stream/351333-glazier/topic/Pointer.20Events)).
    ///
    /// Note: a widget can be hot while another is [`active`] (for example, when
    /// clicking a button and dragging the cursor to another widget).
    ///
    /// [`active`]: Pod::is_active
    pub fn is_hot(&self) -> bool {
        self.state.flags.contains(PodFlags::IS_HOT)
    }

    /// The "active" (aka pressed) status of a widget.
    ///
    /// Active status generally corresponds to a mouse button down. Widgets
    /// with behavior similar to a button will call [`set_active`] on mouse
    /// down and then up.
    ///
    /// The active status can only be set manually. Xilem doesn't automatically
    /// set it to `false` on mouse release or anything like that.
    ///
    /// There is no special handling of the active status for multi-pointer devices. (This is
    /// likely to change in the future as [pointer events are planed](https://xi.zulipchat.com/#narrow/stream/351333-glazier/topic/Pointer.20Events)).
    ///
    /// When a widget is active, it gets mouse events even when the mouse
    /// is dragged away.
    ///
    /// [`set_active`]: EventCx::set_active
    // TODO: Determine why this is the same as [Self::has_active]
    pub fn is_active(&self) -> bool {
        self.state.flags.contains(PodFlags::HAS_ACTIVE)
    }

    /// Returns `true` if any descendant is [`active`].
    ///
    /// [`active`]: Pod::is_active
    pub fn has_active(&self) -> bool {
        self.state.flags.contains(PodFlags::HAS_ACTIVE)
    }

    /// This widget or any of its children have requested layout.
    pub fn layout_requested(&self) -> bool {
        self.state.flags.contains(PodFlags::REQUEST_LAYOUT)
    }
}
