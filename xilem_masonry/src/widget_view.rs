// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{FromDynWidget, HasProperty, Property, Widget};
use masonry::kurbo::Affine;

use crate::core::{View, ViewArgument, ViewSequence};
use crate::view::{Prop, Transformed, transformed};
use crate::{AnyWidgetView, Pod, ViewCtx};

/// The trait for views representing the widget tree.
///
/// This is essentially a "trait wrapper" for `View<S, A, ViewCtx, Element = Pod<impl Widget>> + Send + Sync`.
///
/// Includes helper methods for common operations on widget views.
pub trait WidgetView<State: ViewArgument, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::Widget>> + Send + Sync
{
    /// The widget this view represents.
    type Widget: Widget + FromDynWidget + ?Sized;

    /// Returns a boxed type erased [`AnyWidgetView`]
    ///
    /// # Examples
    /// ```
    /// # use xilem_masonry as xilem;
    /// use xilem::{view::label, WidgetView};
    ///
    /// # fn view<State: xilem::core::ViewArgument>() -> impl WidgetView<State> + use<State> {
    /// label("a label").boxed()
    /// # }
    ///
    /// ```
    fn boxed(self) -> Box<AnyWidgetView<State, Action>>
    where
        Action: 'static,
        Self: Sized,
    {
        Box::new(self)
    }

    /// This widget with a 2d transform applied.
    ///
    /// See [`transformed`] for similar functionality with a builder-API using this.
    /// The return type is the same as for `transformed`, and so also has these
    /// builder methods.
    fn transform(self, by: Affine) -> Transformed<Self, State, Action>
    where
        Self: Sized,
    {
        transformed(self).transform(by)
    }

    /// Set a [`Property`] on this view, when the underlying widget [supports](HasProperty) it.
    ///
    /// This overrides previous set properties of the same type.
    ///
    /// It can be used to create syntax-sugar extension traits with more documentation, as seen in [`Style`](crate::style::Style).
    ///
    /// # Examples
    /// ```
    /// # use xilem_masonry as xilem;
    /// use xilem::{masonry::properties::CornerRadius, view::{text_button, label}, WidgetView};
    ///
    /// # fn view<State: xilem::core::ViewArgument>() -> impl WidgetView<State> + use<State> {
    /// text_button("click me", |_| {})
    ///     .prop(CornerRadius { radius: 20.0 })
    ///     .prop(CornerRadius { radius: 5.0 })
    /// // The corner radius of this button will be 5.0
    /// # }
    ///
    /// ```
    fn prop<P>(self, property: P) -> Prop<P, Self, State, Action>
    where
        State: ViewArgument,
        Action: 'static,
        Self: Sized,
        Self::Widget: HasProperty<P>,
        P: Property + PartialEq + Clone,
    {
        Prop {
            property,
            child: self,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<V, State, Action, W> WidgetView<State, Action> for V
where
    V: View<State, Action, ViewCtx, Element = Pod<W>> + Send + Sync,
    W: Widget + FromDynWidget + ?Sized,
    State: ViewArgument,
{
    type Widget = W;
}

/// An ordered sequence of widget views, it's used for `0..N` views.
/// See [`ViewSequence`] for more technical details.
///
/// # Examples
///
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::{view::prose, WidgetViewSequence, core::ViewArgument};
///
/// fn prose_sequence<State: ViewArgument>(
///     texts: impl Iterator<Item = &'static str>,
/// ) -> impl WidgetViewSequence<State> {
///     texts.map(prose).collect::<Vec<_>>()
/// }
/// ```
pub trait WidgetViewSequence<State: ViewArgument, Action = ()>:
    ViewSequence<State, Action, ViewCtx, Pod<dyn Widget>>
{
}

impl<Seq, State, Action> WidgetViewSequence<State, Action> for Seq
where
    Seq: ViewSequence<State, Action, ViewCtx, Pod<dyn Widget>>,
    State: ViewArgument,
{
}
