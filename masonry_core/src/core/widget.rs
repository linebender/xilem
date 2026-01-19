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
use vello::kurbo::{Axis, Point, Size};

use crate::core::{
    AccessCtx, AccessEvent, ComposeCtx, CursorIcon, EventCtx, Layer, LayoutCtx, MeasureCtx,
    NewWidget, PaintCtx, PointerEvent, Properties, PropertiesMut, PropertiesRef, QueryCtx,
    RegisterCtx, TextEvent, Update, UpdateCtx, WidgetMut, WidgetRef,
};
use crate::layout::LenReq;

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
/// # `WidgetId` cannot be reserved
///
/// Ids are only attributed once a widget is added to the widget tree.
///
/// You can't create a widget with a pre-allocated `WidgetId`.
/// If you want to create a widget in a way that lets you refer to it later,
/// see [`NewWidget::new_with_tag`](crate::core::NewWidget::new_with_tag).
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
    /// Downcasts `widget` if `Self: Sized`, else returns it as-is.
    fn from_dyn(widget: &dyn Widget) -> Option<&Self>;
    /// Downcasts `widget` if `Self: Sized`, else returns it as-is.
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
/// [`measure`](Self::measure), [`layout`](Self::layout), [`compose`](Self::compose),
/// [`paint`](Self::paint), and [`accessibility`](Self::accessibility) are called.
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
/// widget should only be mutated either during a method call or through a [`WidgetMut`].
#[allow(unused_variables, reason = "Default impls don't use method arguments")]
pub trait Widget: AsDynWidget + Any {
    /// The action type that this widget will submit, through [`EventCtx::submit_action`]
    /// (or the method of the same name on a different context).
    /// The type of actions submitted by this widget will be validated against this type.
    ///
    /// If this widget never submits action, this can be an empty type
    /// such as [`NoAction`](crate::core::NoAction).
    type Action: Any + Debug
    where
        Self: Sized;

    /// Handles a pointer event.
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

    /// Handles a text event.
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

    /// Handles an event from the platform's accessibility API.
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

    /// Registers child widgets with Masonry.
    ///
    /// Leaf widgets can implement this with an empty body.
    ///
    /// Container widgets need to call [`RegisterCtx::register_child`] for all
    /// their children. Forgetting to do so is a logic error and may lead to debug panics.
    /// All the children returned by `children_ids` should be visited.
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>);

    /// Handles an update to the widget's state.
    ///
    /// This method is called to notify your widget of certain special events,
    /// (available in the [`Update`] enum) that are generally related to
    /// changes in the widget graph or in the state of your specific widget.
    fn update(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, event: &Update) {}

    // TODO - Remove default implementation
    /// Handles a property being added, changed, or removed.
    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {}

    /// Computes the length that the widget wants to be on the given `axis`.
    ///
    /// The returned length must be finite, non-negative, and in device pixels.
    /// If an invalid length is returned, Masonry will treat it as zero.
    ///
    /// The goal of this method is for a parent to learn how its children want to be sized.
    /// All the inputs are hints towards what the parent is planning for its child.
    /// These hints should be followed, but that is not strictly required. For example,
    /// it is completely valid for the child to always return the same size regardless of space.
    /// Which is to say, at the end of the day, a widget chooses how it measures itself.
    ///
    /// It's a question of preferred size. As `measure` will only be called when there is
    /// no defined length present for the `axis`. So `measure` is not about reading the widget's
    /// [`Dimensions`] from `props` and returning that. That is handled earlier by Masonry.
    /// Instead, it's a task of measuring the contents of the widget. Combining both local state
    /// and child measurements to come up with a answer for the total length of this `axis`.
    ///
    /// Call [`MeasureCtx::compute_length`] to measure children and to get their `axis` length.
    /// Then account for the child's positioning and any other unique layout factors you might have
    /// to determine the total length of this widget on the given `axis`.
    ///
    /// If you have a thin wrapper widget that wants to mimic its child in terms of layout,
    /// then you should use [`MeasureCtx::redirect_measurement`] to have the child answer for you.
    ///
    /// The `cross_length`, if present, says that the cross-axis of the given `axis` of this
    /// measured widget can be presumed to be exactly `cross_length` long, in device pixels.
    /// This information is often very useful for measuring `axis` and should be used.
    /// However, ultimately it may end up not materializing. That is to say, it is
    /// a valid assumption for the duration of this `measure` call but there is
    /// no guarantee that it will still be true when the final size is decided.
    ///
    /// How exactly the length is calculated should be determined based on [`LenReq`].
    /// Adapting to it is a cornerstone of a well running cooperative layout system,
    /// and generally all built-in Masonry widgets will respect the request. However,
    /// ultimately it is completely valid for the widget to return whatever length it wants.
    ///
    /// It is not guaranteed that `measure` will be called even once during a layout pass.
    /// Don't design `measure` to be the only source of critical work that the widget depends on.
    ///
    /// Nor is it guaranteed that a [`layout`] call will follow a `measure` call, because
    /// the parent widget might end up choosing the same size for this widget as last time.
    /// Be very careful with mutating state inside `measure`. You should only mutate state
    /// in a way that is useful for `measure` itself, or in a way that the rest of your widget
    /// will correctly understand. Basically, don't commit any layout choices. You can save
    /// speculative data that [`layout`] might later read and commit. However, generally,
    /// no other part of the widget should be affected by the data that `measure` mutates.
    ///
    /// There might be any number of `measure` calls per layout pass. Depending on the specific
    /// parent's layout algorithm, which might measure its children multiple times. Hence, it's
    /// important for good performance to cache any expensive computations that can be reused.
    ///
    /// Masonry will, by default, cache the results of measurement. The cache key is derived
    /// from `axis`, `len_req`, and `cross_length`. If the widget uses any other data to influence
    /// the result of the measurement, then the widget is responsible for requesting layout
    /// when any of that data changes. For properties, this is handled in [`property_changed`].
    /// For any other data you reference, the exact mechanism of detecting changes is up to you.
    /// Once you've detected a change in that data, call `request_layout` to clear the cache.
    /// If you can't detect changes of the referenced data, disable the cache via [`cache_result`].
    ///
    /// The cache provided by Masonry is small, with only a few entries per widget. If your widget
    /// does expensive computations that aren't necessarily different per the aforementioned inputs,
    /// or you need to request layout for reasons that don't affect the result of this computation,
    /// then your widget should also have its own inner cache layer to avoid redoing the same work.
    ///
    /// As for the inputs provided to `measure`, `len_req` must be [sanitized] and
    /// `cross_length`, if present, must be [sanitized] and in device pixels.
    /// When Masonry calls `measure` during the layout pass, it guarantees that for these inputs.
    ///
    /// # Panics
    ///
    /// Masonry will panic if `measure` returns a non-finite or negative value
    /// and debug assertions are enabled.
    ///
    /// [sanitized]: crate::util::Sanitize
    /// [`cache_result`]: MeasureCtx::cache_result
    /// [`Dimensions`]: crate::properties::Dimensions
    /// [`layout`]: Self::layout
    /// [`property_changed`]: Self::property_changed
    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64;

    /// Lays out the widget with the given `size`.
    ///
    /// A container widget must ensure all its direct children are laid out in this method.
    ///
    /// For every child widget, as defined by [`children_ids`], the container must:
    ///
    /// 1. (Optionally) Call [`LayoutCtx::compute_size`] to get the size the child wants to be.
    ///    If the container has somehow already decided on the child length on one axis, then it
    ///    should instead call [`LayoutCtx::compute_length`] with the correct `cross_length`.
    /// 2. Decide on a final [`Size`] that the child should be. The parent is in control.
    ///    Note, however, that the child will still be in control of its own [`paint`] method.
    ///    If a child is given a size smaller than its [`MinContent`], its painting is likely
    ///    to overflow its bounds, depending on both the child's and the parent's clip settings.
    /// 3. Call [`LayoutCtx::run_layout`] on the child with the chosen size.
    ///    This will recursively trigger the layout pass on both the child and all its descendants.
    /// 4. Call [`LayoutCtx::place_child`] to give the child a location, relative to the parent.
    ///    With that, the laying out of the child is finished.
    ///
    /// The order of laying out children doesn't matter. It is also valid to interleave the calls.
    /// For example you might `compute_size` for a few, lay out one, re-compute the others.
    ///
    /// Failing to lay out and place some child is a logic error and may lead to panics.
    ///
    /// Container widgets must not add or remove children during layout.
    /// Doing so is a logic error and may lead to panics.
    ///
    /// The `size` given to this method must be finite, non-negative, and in device pixels.
    /// When Masonry calls `layout` during the layout pass, it will guarantee that for `size`.
    ///
    /// [`children_ids`]: Self::children_ids
    /// [`paint`]: Self::paint
    /// [`MinContent`]: crate::layout::Dim::MinContent
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size);

    /// Runs after the widget's final transform has been computed.
    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {}

    /// Paints the widget appearance.
    ///
    /// Container widgets can paint a background before recursing to their
    /// children. To draw on top of children, see [`Widget::post_paint`].
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene);

    /// Second paint method, which paints on top of the widget's children.
    ///
    /// This method is not constrained by the clip defined in [`LayoutCtx::set_clip_path`], and can paint things outside the clip.
    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
    }

    /// Returns what kind of "thing" the widget fundamentally is.
    fn accessibility_role(&self) -> Role;

    /// Describes the widget's contents for accessibility APIs.
    ///
    /// This method takes a mutable reference to a node which is already initialized
    /// with some information about the current widget (coordinates, status flags), and
    /// and mutates that node to set widget-specific information.
    ///
    /// **Note:** A new node is created each time this method is called.
    /// Changes to the node don't persist between accessibility passes, and must instead
    /// be re-applied by this method every time it's called.
    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    );

    /// Returns ids of this widget's children.
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

    /// Return `Some(self)` if the widget also implements [`Layer`].
    ///
    /// Default implementation returns `None`.
    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        None
    }

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
    /// Returns a span for tracing.
    ///
    /// As methods recurse through the widget tree, trace spans are added for each child
    /// widget visited, and popped when control flow goes back to the parent. This method
    /// returns a static span (that you can use to filter traces and logs).
    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Widget", r#type = self.short_type_name(), id = id.trace())
    }

    /// Returns a small string representing important info about this widget instance.
    ///
    /// When using [`WidgetRef`]'s [`Debug`] implementation, widgets
    /// will be displayed as a tree of values. Widgets which return a non-null value in
    /// `get_debug_text` will appear with that text next to their type name. This can
    /// be eg a label's text, or whether a checkbox is checked.
    fn get_debug_text(&self) -> Option<String> {
        None
    }

    /// Returns the cursor icon for this widget.
    ///
    /// This will be called when the mouse moves or [`request_cursor_icon_change`](crate::core::MutateCtx::request_cursor_icon_change) is called.
    ///
    /// **pos** - the mouse position in global coordinates (e.g. `(0,0)` is the top-left corner of the
    /// window).
    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        CursorIcon::Default
    }

    // --- Auto-generated implementations ---

    /// Returns the first innermost widget composed by this (including `self`), that contains/intersects with `pos` and accepts pointer interaction, if any.
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

    /// Gets the (verbose) type name of the widget for debugging purposes.
    /// You should not override this method.
    #[doc(hidden)]
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Gets the (abridged) type name of the widget for debugging purposes.
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

    /// Convenience method to wrap this in a [`NewWidget`].
    fn with_auto_id(self) -> NewWidget<Self>
    where
        Self: Sized,
    {
        NewWidget::new(self)
    }

    /// Convenience method to wrap this in a [`NewWidget`] with the given [`Properties`].
    fn with_props(self, props: impl Into<Properties>) -> NewWidget<Self>
    where
        Self: Sized,
    {
        NewWidget::new_with_props(self, props)
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
    /// Allocates a new, unique `WidgetId`.
    ///
    /// All widgets are assigned ids automatically; you should only create
    /// an explicit id if you need to know it ahead of time, for instance
    /// if you want two sibling widgets to know each others' ids.
    ///
    /// You must ensure that a given `WidgetId` is only ever used for one
    /// widget at a time.
    pub(crate) fn next() -> Self {
        static WIDGET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id.try_into().unwrap())
    }

    // TODO - Remove
    // Currently used in Xilem for event routing.
    #[doc(hidden)]
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

/// Trait implemented by collection widgets.
///
/// It provides a standard set of functions to manage children.
pub trait CollectionWidget<Params>: Widget {
    /// Returns the number of children.
    fn len(&self) -> usize;

    /// Returns `true` if there are no children.
    fn is_empty(&self) -> bool;

    /// Returns a mutable reference to the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn get_mut<'t>(this: &'t mut WidgetMut<'_, Self>, idx: usize) -> WidgetMut<'t, dyn Widget>;

    /// Appends a child widget to the collection.
    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<Params>,
    );

    /// Inserts a child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the number of children.
    fn insert(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<Params>,
    );

    /// Replaces the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        params: impl Into<Params>,
    );

    /// Sets the child params at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set_params(this: &mut WidgetMut<'_, Self>, idx: usize, params: impl Into<Params>);

    /// Swaps the index of two children.
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    fn swap(this: &mut WidgetMut<'_, Self>, a: usize, b: usize);

    /// Removes the child at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize);

    /// Removes all children.
    fn clear(this: &mut WidgetMut<'_, Self>);
}
