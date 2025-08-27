// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::properties::types::Length;
use std::marker::PhantomData;

use masonry::widgets;

use crate::core::{MessageContext, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx, WidgetView};

/// A widget with predefined size.
///
/// This widget forces its child to have a specific width and/or height (assuming values are permitted by
/// this widget's parent). If either the width or height is not set, this widget will size itself
/// to match the child's size in that dimension.
///
/// # Example
/// See more methods for `sized_box` on [`SizedBox`] page.
/// ```ignore
/// use xilem::view::{sized_box, button, label};
/// use xilem::palette;
/// use vello::kurbo::RoundedRectRadii;
/// use masonry::properties::Padding;
///
/// sized_box(button(label("Button"), |data: &mut i32| *data+=1))
///     .expand()
///     .background(palette::css::RED)
///     .border(palette::css::YELLOW, 20.)
///     .rounded(RoundedRectRadii::from_single_radius(5.))
///     .padding(Padding::from(5.))
/// ```
pub fn sized_box<State, Action, V>(inner: V) -> SizedBox<V, State, Action>
where
    V: WidgetView<State, Action>,
{
    SizedBox {
        inner,
        height: None,
        width: None,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`sized_box`].
///
/// See `sized_box` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct SizedBox<V, State, Action = ()> {
    inner: V,
    width: Option<f64>,
    height: Option<f64>,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> SizedBox<V, State, Action> {
    /// Set container's width.
    pub fn width(mut self, width: Length) -> Self {
        self.width = Some(width.get());
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: Length) -> Self {
        self.height = Some(height.get());
        self
    }

    /// Expand container to fit the parent.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: Self::expand_height
    /// [`expand_width`]: Self::expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(f64::INFINITY);
        self.height = Some(f64::INFINITY);
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(f64::INFINITY);
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(f64::INFINITY);
        self
    }
}

impl<V, State, Action> ViewMarker for SizedBox<V, State, Action> {}
impl<V, State, Action> View<State, Action, ViewCtx> for SizedBox<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = Pod<widgets::SizedBox>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = self.inner.build(ctx, app_state);
        let widget = widgets::SizedBox::new(child.new_widget)
            .raw_width(self.width)
            .raw_height(self.height);

        (ctx.create_pod(widget), child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if self.width != prev.width {
            widgets::SizedBox::set_raw_width(&mut element, self.width);
        }
        if self.height != prev.height {
            widgets::SizedBox::set_raw_height(&mut element, self.height);
        }
        {
            let mut child = widgets::SizedBox::child_mut(&mut element)
                .expect("We only create SizedBox with a child");
            self.inner
                .rebuild(&prev.inner, view_state, ctx, child.downcast(), app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = widgets::SizedBox::child_mut(&mut element)
            .expect("We only create SizedBox with a child");
        self.inner.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        let mut child = widgets::SizedBox::child_mut(&mut element)
            .expect("We only create SizedBox with a child");
        self.inner
            .message(view_state, message, child.downcast(), app_state)
    }
}
