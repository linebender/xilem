// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use std::any::Any;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

use smallvec::SmallVec;
use tracing::{trace_span, Span};

use crate::event::StatusChange;
use crate::widget::WidgetRef;
use crate::{
    AsAny, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Size, WidgetCtx,
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
/// an id to the WidgetPod you're creating; various widgets which have methods to create
/// children may have variants taking ids as parameters.
///
/// If you set a `WidgetId` directly, you are resposible for ensuring that it
/// is unique. Two widgets must not be created with the same id.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct WidgetId(NonZeroU64);

// TODO - Add tutorial: implementing a widget - See issue #5
/// The trait implemented by all widgets.
///
/// For details on how to implement this trait, see tutorial **(TODO)**
///
/// Whenever external events affect the given widget, methods `[on_event]`,
/// `[on_status_change]` and `[on_lifecycle]` are called. Later on, when the
/// widget is laid out and displayed, methods `[layout]` and `[paint]` are called.
///
/// These trait methods are provided with a corresponding context. The widget can
/// request things and cause actions by calling methods on that context. In
/// addition, these methods are provided with an environment ([`Env`]).
///
/// Widgets also have a `children()` method. Leaf widgets return an empty array,
/// whereas container widgets return an array of [`WidgetRef`]. Container widgets
/// have some validity invariants to maintain regarding their children. See TUTORIAL_2
/// for details **(TODO)**.
///
/// Generally speaking, widgets aren't used directly. They are stored in
/// [`WidgetPods`](crate::WidgetPod). Widget methods are called by WidgetPods, and the
/// widget is mutated either during a method call (eg `on_event` or `lifecycle`) or
/// through a [`WidgetMut`](crate::widget::WidgetMut). See tutorials for detail.
pub trait Widget: AsAny {
    /// Handle an event - usually user interaction.
    ///
    /// A number of different events (in the [`Event`] enum) are handled in this
    /// method call. A widget can handle these events in a number of ways:
    /// requesting things from the [`EventCtx`], mutating the data, or submitting
    /// a [`Command`](crate::Command).
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env);

    #[allow(missing_docs)]
    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env);

    /// Handle a lifecycle notification.
    ///
    /// This method is called to notify your widget of certain special events,
    /// (available in the [`LifeCycle`] enum) that are generally related to
    /// changes in the widget graph or in the state of your specific widget.
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env);

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
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size;

    /// Paint the widget appearance.
    ///
    /// The [`PaintCtx`] derefs to something that implements the
    /// [`piet::RenderContext`](crate::piet::RenderContext) trait, which exposes various methods that the widget
    /// can use to paint its appearance.
    ///
    /// Container widgets can paint a background before recursing to their
    /// children, or annotations (for example, scrollbars) by painting
    /// afterwards. In addition, they can apply masks and transforms on
    /// the render context, which is especially useful for scrolling.
    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env);

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
    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]>;

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
    /// When using [`WidgetRef`]'s [Debug](std::fmt::Debug) implementation, widgets
    /// will be displayed as a tree of values. Widgets which return a non-null value in
    /// `get_debug_text` will appear with that text next to their type name. This can
    /// be eg a label's text, or whether a checkbox is checked.
    fn get_debug_text(&self) -> Option<String> {
        None
    }

    // --- Auto-generated implementations ---

    /// Return which child, if any, has the given `pos` in its layout rect.
    ///
    /// The child return is a direct child, not eg a grand-child. The position is in
    /// relative cordinates. (Eg `(0,0)` is the top-left corner of `self`).
    ///
    /// Has a default implementation, that can be overriden to search children more
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
    /// Cast as Any.
    ///
    /// Mainly intended to be overriden in `Box<dyn Widget>`.
    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any {
        self.as_dyn_any()
    }

    // FIXME
    /// Cast as Any.
    ///
    /// Mainly intended to be overriden in `Box<dyn Widget>`.
    #[doc(hidden)]
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self.as_mut_dyn_any()
    }
}

/// Trait that widgets must implement to be in [`WidgetMut`](crate::widget::WidgetMut).
///
/// This trait should usually be implemented with [`declare_widget`](crate::declare_widget).
#[allow(missing_docs)]
pub trait StoreInWidgetMut: Widget {
    type Mut<'a, 'b: 'a>: Deref<Target = Self>;

    fn from_widget_and_ctx<'a, 'b>(
        widget: &'a mut Self,
        ctx: WidgetCtx<'a, 'b>,
    ) -> Self::Mut<'a, 'b>;

    fn get_widget<'s: 'r, 'a: 'r, 'b: 'a, 'r>(
        widget_mut: &'s mut Self::Mut<'a, 'b>,
    ) -> &'r mut Self {
        Self::get_widget_and_ctx(widget_mut).0
    }

    fn get_ctx<'s: 'r, 'a: 'r, 'b: 'a, 'r>(
        widget_mut: &'s mut Self::Mut<'a, 'b>,
    ) -> &'r mut WidgetCtx<'a, 'b> {
        Self::get_widget_and_ctx(widget_mut).1
    }

    fn get_widget_and_ctx<'s: 'r, 'a: 'r, 'b: 'a, 'r>(
        widget_mut: &'s mut Self::Mut<'a, 'b>,
    ) -> (&'r mut Self, &'r mut WidgetCtx<'a, 'b>);
}

// TODO - Generate a struct instead. See #27.
/// Declare a mutable reference type for your widget.
///
/// The general syntax is:
///
/// ```ignore
/// declare_widget!(MyWidgetMut, MyWidget);
/// ```
///
/// where `MyWidget` is the name of your widget, and `MyWidgetMut` is an arbitrary
/// name (it can be exported for documentation purposes, but it's not going to be
/// instanced directly).
///
/// The above macro call will produce something like this:
///
/// ```ignore
/// pub struct MyWidgetMut<'a, 'b>(WidgetCtx<'a, 'b>, &'a mut MyWidget);
///
/// impl StoreInWidgetMut for MyWidget {
///     type Mut<'a, 'b> = MyWidgetMut<'a, 'b>;
/// }
/// ```
///
/// Because of `WidgetMut`'s [Deref] implementation, any methods implemented on
/// `MyWidgetMut` will thus be usable from `WidgetMut<MyWidget>`.
///
/// **Note:** This is all a huge hack to compensate for the lack of
/// [arbitrary self types](https://github.com/rust-lang/rust/issues/44874). What we
/// would really want is for that feature to be stable.
///
/// ## Generic widgets
///
/// If a widget type has generic arguments, the syntax becomes:
///
/// ```ignore
/// declare_widget!(FoobarMut, Foobar<A, B, C>);
/// ```
///
/// If these arguments have bounds, the syntax becomes:
///
/// ```ignore
/// declare_widget!(FoobarMut, Foobar<A: (SomeTrait), B: (SomeTrait + OtherTrait), C>);
/// ```
///
/// Yes, that is extremely annoying. Sorry about that.
#[macro_export]
macro_rules! declare_widget {
    ($WidgetNameMut:ident, $WidgetName:ident) => {
        $crate::declare_widget!($WidgetNameMut, $WidgetName<>);
    };

    ($WidgetNameMut:ident, $WidgetName:ident<$($Arg:ident $(: ($($Bound:tt)*))?),*>) => {
        pub struct $WidgetNameMut<'a, 'b, $($Arg $(: $($Bound)*)?),*>{
            ctx: $crate::WidgetCtx<'a, 'b>,
            widget: &'a mut $WidgetName<$($Arg),*>
        }

        impl<$($Arg $(: $($Bound)*)?),*> $crate::widget::StoreInWidgetMut for $WidgetName<$($Arg),*> {
            type Mut<'a, 'b: 'a> = $WidgetNameMut<'a, 'b, $($Arg),*>;

            fn get_widget_and_ctx<'s: 'r, 'a: 'r, 'b: 'a, 'r>(
                widget_mut: &'s mut Self::Mut<'a, 'b>,
            ) -> (&'r mut Self, &'r mut $crate::WidgetCtx<'a, 'b>) {
                (widget_mut.widget, &mut widget_mut.ctx)
            }

            fn from_widget_and_ctx<'a, 'b>(
                widget: &'a mut Self,
                ctx: $crate::WidgetCtx<'a, 'b>,
            ) -> Self::Mut<'a, 'b> {
                $WidgetNameMut { ctx, widget }
            }
        }

        impl<'a, 'b, $($Arg $(: $($Bound)*)?),*> ::std::ops::Deref for $WidgetNameMut<'a, 'b, $($Arg),*> {
            type Target = $WidgetName<$($Arg),*>;

            fn deref(&self) -> &Self::Target {
                self.widget
            }
        }
    };
}

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
        use druid_shell::Counter;
        static WIDGET_ID_COUNTER: Counter = Counter::new();
        WidgetId(WIDGET_ID_COUNTER.next_nonzero())
    }

    /// Create a reserved `WidgetId`, suitable for reuse.
    ///
    /// The caller is responsible for ensuring that this ID is in fact assigned
    /// to a single widget at any time, or your code may become haunted.
    ///
    /// The actual inner representation of the returned `WidgetId` will not
    /// be the same as the raw value that is passed in; it will be
    /// `u64::max_value() - raw`.
    #[allow(unsafe_code)]
    pub const fn reserved(raw: u16) -> WidgetId {
        let id = u64::max_value() - raw as u64;
        // safety: by construction this can never be zero.
        WidgetId(unsafe { std::num::NonZeroU64::new_unchecked(id) })
    }

    pub(crate) fn to_raw(self) -> u64 {
        self.0.into()
    }
}

// TODO - remove
impl Widget for Box<dyn Widget> {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        self.deref_mut().on_event(ctx, event, env)
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env) {
        self.deref_mut().on_status_change(ctx, event, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        self.deref_mut().lifecycle(ctx, event, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        self.deref_mut().layout(ctx, bc, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        self.deref_mut().paint(ctx, env);
    }

    fn type_name(&self) -> &'static str {
        self.deref().type_name()
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        self.deref().children()
    }

    fn make_trace_span(&self) -> Span {
        self.deref().make_trace_span()
    }

    fn get_debug_text(&self) -> Option<String> {
        self.deref().get_debug_text()
    }

    fn as_any(&self) -> &dyn Any {
        self.deref().as_dyn_any()
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self.deref_mut().as_mut_dyn_any()
    }
}

// We use alias type because macro doesn't accept braces except in some cases.
type BoxWidget = Box<dyn Widget>;
crate::declare_widget!(BoxWidgetMut, BoxWidget);
