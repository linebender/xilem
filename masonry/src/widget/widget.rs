// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};

use accesskit::Role;
use cursor_icon::CursorIcon;
use smallvec::SmallVec;
use tracing::{trace_span, Span};
use vello::Scene;

use crate::event::{AccessEvent, PointerEvent, StatusChange, TextEvent};
use crate::{
    AccessCtx, AsAny, BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
};

/// A unique identifier for a single [`Widget`].
///
/// `WidgetId`s are generated automatically for all widgets in the widget tree.
/// More specifically, each [`WidgetPod`](crate::WidgetPod) has a unique `WidgetId`.
///
/// These ids are used internally to route events, and can be used to communicate
/// between widgets, by submitting a command (as with [`EventCtx::submit_command`])
/// and passing a `WidgetId` as the [`Target`](crate::Target).
///
/// A widget can retrieve its id via methods on the various contexts, such as
/// [`LifeCycleCtx::widget_id`].
///
/// ## Explicit `WidgetId`s.
///
/// Sometimes, you may want to construct a widget, in a way that lets you know its id,
/// so you can refer to the widget later. You can use [`WidgetPod::new_with_id`](crate::WidgetPod::new_with_id) to pass
/// an id to the `WidgetPod` you're creating; various widgets which have methods to create
/// children may have variants taking ids as parameters.
///
/// If you set a `WidgetId` directly, you are responsible for ensuring that it
/// is unique. Two widgets must not be created with the same id.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct WidgetId(pub(crate) NonZeroU64);

// TODO - Add tutorial: implementing a widget - See https://github.com/linebender/xilem/issues/376
/// The trait implemented by all widgets.
///
/// For details on how to implement this trait, see tutorial **(TODO)**
///
/// Whenever external events affect the given widget, methods [`on_event`],
/// [`on_status_change`](Self::on_status_change) and [`lifecycle`](Self::lifecycle)
/// are called. Later on, when the widget is laid out and displayed, methods
/// [`layout`](Self::layout) and [`paint`](Self::paint) are called.
///
/// These trait methods are provided with a corresponding context. The widget can
/// request things and cause actions by calling methods on that context.
///
/// Widgets also have a [`children`](Self::children) method. Leaf widgets return an empty array,
/// whereas container widgets return an array of [`WidgetRef`]. Container widgets
/// have some validity invariants to maintain regarding their children. See TUTORIAL_2
/// for details **(TODO)**.
///
/// Generally speaking, widgets aren't used directly. They are stored in
/// [`WidgetPod`](crate::WidgetPod)s. Widget methods are called by `WidgetPod`s, and the
/// widget is mutated either during a method call (eg `on_event` or `lifecycle`) or
/// through a [`WidgetMut`](crate::widget::WidgetMut). See tutorials for details.
pub trait Widget: AsAny {
    /// Handle an event - usually user interaction.
    ///
    /// A number of different events (in the [`Event`] enum) are handled in this
    /// method call. A widget can handle these events in a number of ways, such as
    /// requesting things from the [`EventCtx`] or mutating the data.
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent);
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent);

    /// Handle an event from the platform's accessibility API.
    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent);

    #[allow(missing_docs)]
    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange);

    /// Handle a lifecycle notification.
    ///
    /// This method is called to notify your widget of certain special events,
    /// (available in the [`LifeCycle`] enum) that are generally related to
    /// changes in the widget graph or in the state of your specific widget.
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle);

    /// Compute layout.
    ///
    /// A leaf widget should determine its size (subject to the provided
    /// constraints) and return it.
    ///
    /// A container widget will recursively call [`WidgetPod::layout`](crate::WidgetPod::layout) on its
    /// child widgets, providing each of them an appropriate box constraint,
    /// compute layout, then call [`LayoutCtx::place_child`] on each of its children.
    /// Finally, it should return the size of the container. The container
    /// can recurse in any order, which can be helpful to, for example, compute
    /// the size of non-flex widgets first, to determine the amount of space
    /// available for the flex widgets.
    ///
    /// For efficiency, a container should only invoke layout of a child widget
    /// once, though there is nothing enforcing this.
    ///
    /// The layout strategy is strongly inspired by Flutter.
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size;

    /// Paint the widget appearance.
    ///
    /// Container widgets can paint a background before recursing to their
    /// children, or annotations (for example, scrollbars) by painting
    /// afterwards. In addition, they can apply masks and transforms on
    /// the render context, which is especially useful for scrolling.
    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene);

    fn accessibility_role(&self) -> Role;

    fn accessibility(&mut self, ctx: &mut AccessCtx);

    /// Return references to this widget's children.
    ///
    /// Leaf widgets return an empty array. Container widgets return references to
    /// their children.
    ///
    /// This methods has some validity invariants. A widget's children list must be
    /// consistent. If children are added or removed, the parent widget should call
    /// `children_changed` on one of the Ctx parameters. Container widgets are also
    /// responsible for calling the main methods (on_event, lifecycle, layout, paint)
    /// on their children.
    /// TODO - Update this doc
    fn children_ids(&self) -> SmallVec<[WidgetId; 16]>;

    // TODO - Rename
    // TODO - Document
    fn skip_pointer(&self) -> bool {
        false
    }

    /// Return a span for tracing.
    ///
    /// As methods recurse through the widget tree, trace spans are added for each child
    /// widget visited, and popped when control flow goes back to the parent. This method
    /// returns a static span (that you can use to filter traces and logs).
    fn make_trace_span(&self) -> Span {
        trace_span!("Widget", r#type = self.short_type_name())
    }

    /// Return a small string representing important info about this widget instance.
    ///
    /// When using [`WidgetRef`]'s [`Debug`](std::fmt::Debug) implementation, widgets
    /// will be displayed as a tree of values. Widgets which return a non-null value in
    /// `get_debug_text` will appear with that text next to their type name. This can
    /// be eg a label's text, or whether a checkbox is checked.
    fn get_debug_text(&self) -> Option<String> {
        None
    }

    // TODO - Document
    // TODO - Add &UpdateCtx argument
    fn get_cursor(&self) -> CursorIcon {
        CursorIcon::Default
    }

    // --- Auto-generated implementations ---

    // FIXME
    #[cfg(FALSE)]
    /// Return which child, if any, has the given `pos` in its layout rect.
    ///
    /// The child return is a direct child, not eg a grand-child. The position is in
    /// relative coordinates. (Eg `(0,0)` is the top-left corner of `self`).
    ///
    /// Has a default implementation, that can be overridden to search children more
    /// efficiently.
    fn get_child_at_pos(&self, pos: Point) -> Option<WidgetRef<'_, dyn Widget>> {
        // layout_rect() is in parent coordinate space
        self.children()
            .into_iter()
            .find(|child| child.state().layout_rect().contains(pos))
    }

    /// Get the (verbose) type name of the widget for debugging purposes.
    /// You should not override this method.
    #[doc(hidden)]
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Get the (abridged) type name of the widget for debugging purposes.
    /// You should not override this method.
    #[doc(hidden)]
    fn short_type_name(&self) -> &'static str {
        let name = self.type_name();
        name.split('<')
            .next()
            .unwrap_or(name)
            .split("::")
            .last()
            .unwrap_or(name)
    }

    // FIXME
    /// Cast as `Any`.
    ///
    /// Mainly intended to be overridden in `Box<dyn Widget>`.
    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any {
        self.as_dyn_any()
    }

    // FIXME
    /// Cast as `Any`.
    ///
    /// Mainly intended to be overridden in `Box<dyn Widget>`.
    #[doc(hidden)]
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self.as_mut_dyn_any()
    }
}

/// Marker trait for Widgets whose parents can get a raw mutable reference to them.
///
/// "Raw mut" means using a mutable reference (eg `&mut MyWidget`) to the data
/// structure, instead of going through the Widget trait methods
/// (`on_text_event`, `lifecycle`, `layout`, etc) or through `WidgetMut`.
///
/// A parent widget can use [`EventCtx::get_raw_mut`], [`LifeCycleCtx::get_raw_mut`],
/// or [`LayoutCtx::get_raw_mut`] to directly access a child widget. In that case,
/// these methods return both a mutable reference to the child widget and a new
/// context (`MutateCtx`, `EventCtx`, etc) scoped to the child. The parent is
/// responsible for calling the context methods (eg `request_layout`,
/// `request_accessibility_update`) for the child.
///
/// Widgets implementing `AllowRawMut` are usually private widgets used as an
/// internal implementation detail of public widgets.
pub trait AllowRawMut: Widget {}

#[cfg(not(tarpaulin_include))]
impl WidgetId {
    /// Allocate a new, unique `WidgetId`.
    ///
    /// All widgets are assigned ids automatically; you should only create
    /// an explicit id if you need to know it ahead of time, for instance
    /// if you want two sibling widgets to know each others' ids.
    ///
    /// You must ensure that a given `WidgetId` is only ever used for one
    /// widget at a time.
    pub fn next() -> WidgetId {
        static WIDGET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        WidgetId(id.try_into().unwrap())
    }

    // TODO - Remove
    /// Create a reserved `WidgetId`, suitable for reuse.
    ///
    /// The caller is responsible for ensuring that this ID is in fact assigned
    /// to a single widget at any time, or your code may become haunted.
    ///
    /// The actual inner representation of the returned `WidgetId` will not
    /// be the same as the raw value that is passed in; it will be
    /// `u64::max_value() - raw`.
    #[allow(clippy::missing_panics_doc)] // Can never panic
    pub const fn reserved(raw: u16) -> WidgetId {
        let id = u64::MAX - raw as u64;
        match NonZeroU64::new(id) {
            Some(id) => WidgetId(id),
            // panic safety: u64::MAX - any u16 can never be zero
            None => unreachable!(),
        }
    }

    pub fn to_raw(self) -> u64 {
        self.0.into()
    }
}

impl From<WidgetId> for accesskit::NodeId {
    fn from(id: WidgetId) -> accesskit::NodeId {
        accesskit::NodeId(id.0.into())
    }
}

#[warn(clippy::missing_trait_methods)]
// TODO - remove
impl Widget for Box<dyn Widget> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        self.deref_mut().on_pointer_event(ctx, event);
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.deref_mut().on_text_event(ctx, event);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        self.deref_mut().on_access_event(ctx, event);
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        self.deref_mut().on_status_change(ctx, event);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.deref_mut().lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        self.deref_mut().layout(ctx, bc)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.deref_mut().paint(ctx, scene);
    }

    fn accessibility_role(&self) -> Role {
        self.deref().accessibility_role()
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        self.deref_mut().accessibility(ctx);
    }

    fn type_name(&self) -> &'static str {
        self.deref().type_name()
    }

    fn short_type_name(&self) -> &'static str {
        self.deref().short_type_name()
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.deref().children_ids()
    }

    fn skip_pointer(&self) -> bool {
        self.deref().skip_pointer()
    }

    fn make_trace_span(&self) -> Span {
        self.deref().make_trace_span()
    }

    fn get_debug_text(&self) -> Option<String> {
        self.deref().get_debug_text()
    }

    fn get_cursor(&self) -> CursorIcon {
        self.deref().get_cursor()
    }

    fn as_any(&self) -> &dyn Any {
        self.deref().as_any()
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self.deref_mut().as_mut_any()
    }
}
