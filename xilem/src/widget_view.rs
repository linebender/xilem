// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::kurbo::Affine;

use masonry::core::{FromDynWidget, HasProperty, Property, Widget};

use crate::core::{View, ViewSequence};
use crate::view::{Prop, Transformed, transformed};
use crate::{AnyWidgetView, Pod, ViewCtx};

#[expect(missing_docs, reason = "TODO - Document these items")]
pub trait WidgetView<State, Action = ()>:
    View<State, Action, ViewCtx, Element = Pod<Self::Widget>> + Send + Sync
{
    type Widget: Widget + FromDynWidget + ?Sized;

    /// Returns a boxed type erased [`AnyWidgetView`]
    ///
    /// # Examples
    /// ```
    /// use xilem::{view::label, WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> + use<State> {
    /// label("a label").boxed()
    /// # }
    ///
    /// ```
    fn boxed(self) -> Box<AnyWidgetView<State, Action>>
    where
        State: 'static,
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

    /// Set a [Property] on this view, when the underlying widget [supports](HasProperty) it.
    ///
    /// This overrides previous set properties of the same type.
    ///
    /// It can be used to create syntax-sugar extension traits with more documentation, as seen in [Style](crate::style::Style)
    ///
    /// # Examples
    /// ```
    /// use xilem::{masonry::properties::CornerRadius, view::{button, label}, WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> + use<State> {
    /// button(label("click me"), |_| {})
    ///     .prop(CornerRadius { radius: 20.0 })
    ///     .prop(CornerRadius { radius: 5.0 })
    /// // The corner radius of this button will be 5.0
    /// # }
    ///
    /// ```
    fn prop<P: Property>(self, property: P) -> Prop<P, Self, State, Action>
    where
        State: 'static,
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
{
    type Widget = W;
}

/// An ordered sequence of widget views, it's used for `0..N` views.
/// See [`ViewSequence`] for more technical details.
///
/// # Examples
///
/// ```
/// use xilem::{view::prose, WidgetViewSequence};
///
/// fn prose_sequence<State: 'static>(
///     texts: impl Iterator<Item = &'static str>,
/// ) -> impl WidgetViewSequence<State> {
///     texts.map(prose).collect::<Vec<_>>()
/// }
/// ```
pub trait WidgetViewSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, Pod<dyn Widget>>
{
}

impl<Seq, State, Action> WidgetViewSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, Pod<dyn Widget>>
{
}
