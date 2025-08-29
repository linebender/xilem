// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::{Any, TypeId};
use std::fmt::{Debug, Display};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use accesskit::{Node, Role};
use smallvec::SmallVec;
use tracing::field::DisplayValue;
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Point, Size};

use crate::core::{
    AccessCtx, AccessEvent, BoxConstraints, ComposeCtx, CursorIcon, EventCtx, LayoutCtx, NewWidget,
    PaintCtx, PointerEvent, Properties, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx,
    TextEvent, Update, UpdateCtx, WidgetRef,
};

/// A unique identifier for a single [`Widget`].
///
/// `WidgetId`s are generated automatically for all widgets in the widget tree.
/// More specifically, each [`WidgetPod`](crate::core::WidgetPod) has a unique `WidgetId`.
///
/// These ids are used internally to route events, and can be used to fetch a specific
/// widget for testing or event handling.
///
/// A widget can retrieve its id via methods on the various contexts, such as
/// [`UpdateCtx::widget_id`].
///
/// # Explicit `WidgetId`s.
///
/// Sometimes, you may want to construct a widget, in a way that lets you know its id,
/// so you can refer to the widget later. You can use [`NewWidget::new_with_id`](crate::core::NewWidget::new_with_id) to pass
/// an id to the `NewWidget` you're creating; various widgets which have methods to create
/// children may have variants taking ids as parameters.
///
/// If you set a `WidgetId` directly, you are responsible for ensuring that it
/// is unique. Two widgets must not be created with the same id.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct WidgetId(pub(crate) NonZeroU64);

impl WidgetId {
    /// A serialized representation of the `WidgetId` for debugging purposes.
    pub fn trace(self) -> DisplayValue<Self> {
        tracing::field::display(self)
    }
}

#[doc(hidden)]
/// A trait to access a [`Widget`] value as a trait object. It is implemented for all types that implement `Widget`.
pub trait AsDynWidget {
    fn as_box_dyn(self: Box<Self>) -> Box<dyn Widget>;
    fn as_dyn(&self) -> &dyn Widget;
    fn as_mut_dyn(&mut self) -> &mut dyn Widget;
}

impl<T: Widget> AsDynWidget for T {
    fn as_box_dyn(self: Box<Self>) -> Box<dyn Widget> {
        self
    }

    fn as_dyn(&self) -> &dyn Widget {
        self as &dyn Widget
    }

    fn as_mut_dyn(&mut self) -> &mut dyn Widget {
        self as &mut dyn Widget
    }
}

/// A trait that lets functions either downcast to a `Sized` widget or keep a `dyn Widget`.
pub trait FromDynWidget {
    /// Downcast `widget` if `Self: Sized`, else return it as-is.
    fn from_dyn(widget: &dyn Widget) -> Option<&Self>;
    /// Downcast `widget` if `Self: Sized`, else return it as-is.
    fn from_dyn_mut(widget: &mut dyn Widget) -> Option<&mut Self>;
}

impl<T: Widget> FromDynWidget for T {
    fn from_dyn(widget: &dyn Widget) -> Option<&Self> {
        (widget as &dyn Any).downcast_ref()
    }

    fn from_dyn_mut(widget: &mut dyn Widget) -> Option<&mut Self> {
        (widget as &mut dyn Any).downcast_mut()
    }
}

impl FromDynWidget for dyn Widget {
    fn from_dyn(widget: &dyn Widget) -> Option<&Self> {
        Some(widget)
    }

    fn from_dyn_mut(widget: &mut dyn Widget) -> Option<&mut Self> {
        Some(widget)
    }
}

/// A collection of widget ids, to be returned from [`Widget::children_ids`].
///
/// Internally, this uses a small vector optimisation, but you should treat it as an append-only `Vec<WidgetId>`.
/// You can use `ChildrenIds::from_slice` with an array to make a list of children ids of known size,
/// or use `ChildrenIds::new` then `push` to it.
/// This type also implements [`FromIterator<WidgetId>`](core::iter::FromIterator).
// TODO: Consider making our own wrapper type here, to make future breaking changes easier.?
pub type ChildrenIds = SmallVec<[WidgetId; 16]>;

/// The trait implemented by all widgets.
///
/// For details on how to implement this trait, see the [tutorials](crate::doc).
///
/// Whenever external events affect the given widget, methods
/// [`on_pointer_event`](Self::on_pointer_event),
/// [`on_text_event`](Self::on_text_event),
/// [`on_access_event`](Self::on_access_event),
/// [`on_anim_frame`](Self::on_anim_frame) and [`update`](Self::update) are called.
///
/// Later on, when the widget is laid out and displayed, methods
/// [`layout`](Self::layout), [`compose`](Self::compose), [`paint`](Self::paint) and
/// [`accessibility`](Self::accessibility) are called.
///
/// These trait methods are provided with a corresponding context. The widget can
/// request things and cause actions by calling methods on that context.
///
/// Widgets also have a [`children_ids`](Self::children_ids) method. Leaf widgets return an empty array,
/// whereas container widgets return an array of [`WidgetId`].
/// Container widgets have some validity invariants to maintain regarding their children.
///
/// Generally speaking, widgets aren't used directly. They are stored by Masonry and accessed
/// through [`WidgetPod`](crate::core::WidgetPod)s. Widget methods are called by Masonry, and a
/// widget should only be mutated either during a method call or through a [`WidgetMut`](crate::core::WidgetMut).
#[allow(unused_variables, reason = "Default impls don't use method arguments")]
pub trait Widget: AsDynWidget + Any {
    /// The action type that will widget will submit.
    ///
    /// If this widget never submits action, this can be an empty type
    /// such as [`NoAction`](crate::core::NoAction).
    type Action: Any + Debug
    where
        Self: Sized;

    /// Handle a pointer event.
    ///
    /// Pointer events will target the widget under the pointer, and then the
    /// event will bubble to each of its parents.
    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
    }

    /// Handle a text event.
    ///
    /// Text events will target the [focused widget], then bubble to each parent.
    ///
    /// [focused widget]: crate::doc::masonry_concepts#text-focus
    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
    }

    /// Handle an event from the platform's accessibility API.
    ///
    /// Accessibility events target a specific widget id, then bubble to each parent.
    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
    }

    /// Called at the beginning of a new animation frame.
    ///
    /// An animation frame does not implicitly request a repaint of this widget.
    /// That is, if you change something which changes how this widget is
    /// drawn, you should call
    /// [`request_paint_only`](UpdateCtx::request_paint_only)
    /// ([`request_render`](UpdateCtx::request_render) if an accessibility
    /// update is also required). This method should itself call
    /// [`request_anim`](UpdateCtx::request_anim_frame) unless the animation
    /// has finished.
    ///
    /// On the first frame when transitioning from idle to animating, `interval`
    /// will be 0. (This logic is presently per-window but might change to
    /// per-widget to make it more consistent). Otherwise it is in nanoseconds.
    ///
    /// The `paint` method will often be called shortly after this event is finished.
    /// For that reason, you should try to avoid doing anything computationally
    /// intensive in response to an `AnimFrame` event: it might make the app miss
    /// the monitor's refresh, causing lag or jerky animations.
    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
    }

    // TODO - Reorder methods to match 02_implementing_widget.md

    /// Register child widgets with Masonry.
    ///
    /// Leaf widgets can implement this with an empty body.
    ///
    /// Container widgets need to call [`RegisterCtx::register_child`] for all
    /// their children. Forgetting to do so is a logic error and may lead to debug panics.
    /// All the children returned by `children_ids` should be visited.
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>);

    /// Handle an update to the widget's state.
    ///
    /// This method is called to notify your widget of certain special events,
    /// (available in the [`Update`] enum) that are generally related to
    /// changes in the widget graph or in the state of your specific widget.
    fn update(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, event: &Update) {}

    // TODO - Remove default implementation
    /// Handle a property being added, changed, or removed.
    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {}

    /// Compute layout and return the widget's size.
    ///
    /// A leaf widget should determine its size (subject to the provided
    /// constraints) and return it.
    ///
    /// A container widget will recursively call [`LayoutCtx::run_layout`] on its
    /// child widgets, providing each of them an appropriate box constraint,
    /// run some layout logic, then call [`LayoutCtx::place_child`] on each of its children.
    /// Finally, it should return the size of the container. The container
    /// can recurse in any order, which can be helpful to, for example, compute
    /// the size of non-flex widgets first, to determine the amount of space
    /// available for the flex widgets.
    ///
    /// Forgetting to visit children is a logic error and may lead to debug panics.
    /// All the children returned by `children_ids` should be visited.
    ///
    /// For efficiency, a container should only invoke layout of a child widget
    /// once, though there is nothing enforcing this.
    ///
    /// **Container widgets should not add or remove children during layout.**
    /// Doing so is a logic error and may trigger a debug assertion.
    ///
    /// While each widget should try to return a size that fits the input constraints,
    /// **any widget may return a size that doesn't fit its constraints**, and container
    /// widgets should handle those cases gracefully.
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size;

    /// Runs after the widget's final transform has been computed.
    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {}

    /// Paint the widget appearance.
    ///
    /// Container widgets can paint a background before recursing to their
    /// children. To draw on top of children, see [`Widget::post_paint`].
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene);

    /// Second paint method, which paints on top of the widget's children.
    ///
    /// This method is not constrained by the clip defined in [`LayoutCtx::set_clip_path`], and can paint things outside the clip.
    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
    }

    /// Return what kind of "thing" the widget fundamentally is.
    fn accessibility_role(&self) -> Role;

    /// Describe the widget's contents for accessibility APIs.
    ///
    /// This method takes a mutable reference to a node which is already initialized
    /// with some information about the current widget (coordinates, status flags), and
    /// and mutates that node to set widget-specific information.
    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    );

    /// Return ids of this widget's children.
    ///
    /// Leaf widgets return an empty array. Container widgets return ids of
    /// their children.
    ///
    /// The list returned by this method is considered the "canonical" list of children
    /// by Masonry.
    ///
    /// This method has some validity invariants. A widget's children list must be
    /// consistent. If children are added or removed, the parent widget should call
    /// `children_changed` on one of the Ctx parameters. Container widgets are
    /// responsible for visiting all their children during `layout` and `register_children`.
    fn children_ids(&self) -> ChildrenIds;

    /// Whether this widget gets pointer events and [hovered] status. True by default.
    ///
    /// If false, the widget will be treated as "transparent" for the pointer, meaning
    /// that the pointer will be considered as hovering whatever is under this widget.
    ///
    /// **Note:** The value returned by this method is cached at widget creation and can't be changed.
    ///
    /// [hovered]: crate::doc::masonry_concepts#hovered
    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    /// Whether this widget gets [text focus]. False by default.
    ///
    /// If true, pressing Tab can focus this widget.
    ///
    /// **Note:** The value returned by this method is cached at widget creation and can't be changed.
    ///
    /// [text focus]: crate::doc::masonry_concepts#text-focus
    fn accepts_focus(&self) -> bool {
        false
    }

    /// Whether this widget gets IME events. False by default.
    ///
    /// If true, focusing this widget will start an IME session.
    ///
    /// **Note:** The value returned by this method is cached at widget creation and can't be changed.
    fn accepts_text_input(&self) -> bool {
        false
    }

    // TODO - Write a generic default implementation once
    // `const std::any::type_name` is stable.
    // See https://github.com/rust-lang/rust/issues/63084
    /// Return a span for tracing.
    ///
    /// As methods recurse through the widget tree, trace spans are added for each child
    /// widget visited, and popped when control flow goes back to the parent. This method
    /// returns a static span (that you can use to filter traces and logs).
    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Widget", r#type = self.short_type_name(), id = id.trace())
    }

    /// Return a small string representing important info about this widget instance.
    ///
    /// When using [`WidgetRef`]'s [`Debug`] implementation, widgets
    /// will be displayed as a tree of values. Widgets which return a non-null value in
    /// `get_debug_text` will appear with that text next to their type name. This can
    /// be eg a label's text, or whether a checkbox is checked.
    fn get_debug_text(&self) -> Option<String> {
        None
    }

    /// Return the cursor icon for this widget.
    ///
    /// This will be called when the mouse moves or [`request_cursor_icon_change`](crate::core::MutateCtx::request_cursor_icon_change) is called.
    ///
    /// **pos** - the mouse position in global coordinates (e.g. `(0,0)` is the top-left corner of the
    /// window).
    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        CursorIcon::Default
    }

    // --- Auto-generated implementations ---

    /// Return the first innermost widget composed by this (including `self`), that contains/intersects with `pos` and accepts pointer interaction, if any.
    ///
    /// In case of overlapping children, the last child as determined by [`Widget::children_ids`] is chosen. No widget is
    /// returned if `pos` is outside the widget's clip path.
    ///
    /// Has a default implementation that can be overridden to search children more efficiently.
    /// Custom implementations must uphold the conditions outlined above.
    ///
    /// **pos** - the position in global coordinates (e.g. `(0,0)` is the top-left corner of the
    /// window).
    fn find_widget_under_pointer<'c>(
        &'c self,
        ctx: QueryCtx<'c>,
        pos: Point,
    ) -> Option<WidgetRef<'c, dyn Widget>> {
        find_widget_under_pointer(self.as_dyn(), ctx, pos)
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

    /// Convenience method to create wrap this in a [`NewWidget`].
    fn with_auto_id(self) -> NewWidget<Self>
    where
        Self: Sized,
    {
        NewWidget::new(self)
    }

    // TODO - We eventually want to remove the ability to reserve widget ids.
    // See https://github.com/linebender/xilem/issues/1255
    /// Convenience method to create wrap this in a [`NewWidget`] with the given id.
    fn with_id(self, id: WidgetId) -> NewWidget<Self>
    where
        Self: Sized,
    {
        NewWidget::new_with_id(self, id)
    }

    /// Convenience method to create wrap this in a [`NewWidget`] with the given [`Properties`].
    fn with_props(self, props: Properties) -> NewWidget<Self>
    where
        Self: Sized,
    {
        NewWidget::new_with(self, WidgetId::next(), false, props)
    }
}

/// See [`Widget::find_widget_under_pointer`] for more details.
pub fn find_widget_under_pointer<'c>(
    widget: &'c dyn Widget,
    ctx: QueryCtx<'c>,
    pos: Point,
) -> Option<WidgetRef<'c, dyn Widget>> {
    if !ctx.bounding_rect().contains(pos) {
        return None;
    }
    if ctx.is_stashed() {
        return None;
    }

    let local_pos = ctx.window_transform().inverse() * pos;

    if let Some(clip) = ctx.clip_path()
        && !clip.contains(local_pos)
    {
        return None;
    }

    // Assumes `Self::children_ids` is in increasing "z-order", picking the last child in case
    // of overlapping children.
    for child_id in widget.children_ids().iter().rev() {
        let child_ref = ctx.get(*child_id);
        if let Some(child) = child_ref
            .widget
            .find_widget_under_pointer(child_ref.ctx, pos)
        {
            return Some(child);
        }
    }

    // If no child is under pointer, test the current widget.
    if ctx.accepts_pointer_interaction() && ctx.size().to_rect().contains(local_pos) {
        Some(WidgetRef { widget, ctx })
    } else {
        None
    }
}

/// Marker trait for widgets whose parents can get a raw mutable reference to them.
///
/// "Raw mut" means using a mutable reference (eg `&mut MyWidget`) to the data
/// structure, instead of going through the [`Widget`] trait methods
/// (`on_text_event`, `update`, `layout`, etc) or through `WidgetMut`.
///
/// A parent widget can use [`EventCtx::get_raw_mut`], [`UpdateCtx::get_raw_mut`],
/// or [`LayoutCtx::get_raw_mut`] to directly access a child widget. In that case,
/// these methods return both a mutable reference to the child widget and a new
/// [`RawCtx`](crate::core::RawCtx) context scoped to the child. The parent is
/// responsible for calling the context methods (eg `request_layout`,
/// `request_accessibility_update`) for the child.
///
/// Widgets implementing `AllowRawMut` are usually private widgets used as an
/// internal implementation detail of public widgets.
pub trait AllowRawMut: Widget {}

impl WidgetId {
    /// Allocate a new, unique `WidgetId`.
    ///
    /// All widgets are assigned ids automatically; you should only create
    /// an explicit id if you need to know it ahead of time, for instance
    /// if you want two sibling widgets to know each others' ids.
    ///
    /// You must ensure that a given `WidgetId` is only ever used for one
    /// widget at a time.
    pub fn next() -> Self {
        static WIDGET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id.try_into().unwrap())
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
    pub const fn reserved(raw: u16) -> Self {
        let id = u64::MAX - raw as u64;
        match NonZeroU64::new(id) {
            Some(id) => Self(id),
            // panic safety: u64::MAX - any u16 can never be zero
            None => unreachable!(),
        }
    }

    /// Returns the integer value of the `WidgetId`.
    pub fn to_raw(self) -> u64 {
        self.0.into()
    }
}

impl From<WidgetId> for u64 {
    fn from(id: WidgetId) -> Self {
        id.0.into()
    }
}

impl From<WidgetId> for accesskit::NodeId {
    fn from(id: WidgetId) -> Self {
        Self(id.0.into())
    }
}

impl Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl PartialEq<accesskit::NodeId> for WidgetId {
    fn eq(&self, other: &accesskit::NodeId) -> bool {
        self.to_raw() == other.0
    }
}

impl PartialEq<WidgetId> for accesskit::NodeId {
    fn eq(&self, other: &WidgetId) -> bool {
        self.0 == other.to_raw()
    }
}
