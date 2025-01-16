// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::widget;
pub use masonry::widget::Padding;
use vello::kurbo::RoundedRectRadii;
use vello::peniko::Brush;

use crate::core::{DynMessage, Mut, View, ViewId, ViewMarker};
use crate::{Affine, Pod, ViewCtx, WidgetView};

use super::Transformable;

/// A widget with predefined size.
///
/// This widget forces its child to have a specific width and/or height (assuming values are permitted by
/// this widget's parent). If either the width or height is not set, this widget will size itself
/// to match the child's size in that dimension.
///
/// # Example
/// See more methods for `sized_box` on [`SizedBox`] page.
/// ```ignore
/// use xilem::view::{sized_box, button};
/// use xilem::palette;
/// use vello::kurbo::RoundedRectRadii;
/// use masonry::widget::Padding;
///
/// sized_box(button("Button", |data: &mut i32| *data+=1))
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
        background: None,
        border: None,
        corner_radius: RoundedRectRadii::from_single_radius(0.0),
        padding: Padding::ZERO,
        phantom: PhantomData,
        transform: Affine::IDENTITY,
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
    background: Option<Brush>,
    border: Option<BorderStyle>,
    corner_radius: RoundedRectRadii,
    padding: Padding,
    phantom: PhantomData<fn() -> (State, Action)>,
    transform: Affine,
}

impl<V, State, Action> SizedBox<V, State, Action> {
    /// Set container's width.
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
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

    /// Builder-style method for setting the background for this widget.
    ///
    /// This can be passed anything which can be represented by a [`Brush`];
    /// notably, it can be any [`Color`], any gradient, or an [`Image`].
    ///
    /// [`Color`]: crate::Color
    /// [`Image`]: vello::peniko::Image
    pub fn background(mut self, brush: impl Into<Brush>) -> Self {
        self.background = Some(brush.into());
        self
    }

    /// Builder-style method for painting a border around the widget with a brush and width.
    pub fn border(mut self, brush: impl Into<Brush>, width: impl Into<f64>) -> Self {
        self.border = Some(BorderStyle {
            brush: brush.into(),
            width: width.into(),
        });
        self
    }

    /// Builder style method for rounding off corners of this container by setting a corner radius.
    pub fn rounded(mut self, radius: impl Into<RoundedRectRadii>) -> Self {
        self.corner_radius = radius.into();
        self
    }

    /// Builder style method for adding a padding around the widget.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }
}

impl<V, State, Action> Transformable for SizedBox<V, State, Action> {
    fn transform_mut(&mut self) -> &mut Affine {
        &mut self.transform
    }
}

impl<V, State, Action> ViewMarker for SizedBox<V, State, Action> {}
impl<V, State, Action> View<State, Action, ViewCtx> for SizedBox<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = Pod<widget::SizedBox>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = self.inner.build(ctx);
        let mut widget = widget::SizedBox::new_pod(child.inner.boxed())
            .raw_width(self.width)
            .raw_height(self.height)
            .rounded(self.corner_radius)
            .padding(self.padding);
        if let Some(background) = &self.background {
            widget = widget.background(background.clone());
        }
        if let Some(border) = &self.border {
            widget = widget.border(border.brush.clone(), border.width);
        }
        let pod = ctx.new_pod_with_transform(widget, self.transform);
        (pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        if prev.transform != self.transform {
            element.set_transform(self.transform);
        }
        if self.width != prev.width {
            match self.width {
                Some(width) => widget::SizedBox::set_width(&mut element, width),
                None => widget::SizedBox::unset_width(&mut element),
            }
        }
        if self.height != prev.height {
            match self.height {
                Some(height) => widget::SizedBox::set_height(&mut element, height),
                None => widget::SizedBox::unset_height(&mut element),
            }
        }
        if self.background != prev.background {
            match &self.background {
                Some(background) => {
                    widget::SizedBox::set_background(&mut element, background.clone());
                }
                None => widget::SizedBox::clear_background(&mut element),
            }
        }
        if self.border != prev.border {
            match &self.border {
                Some(border) => {
                    widget::SizedBox::set_border(&mut element, border.brush.clone(), border.width);
                }
                None => widget::SizedBox::clear_border(&mut element),
            }
        }
        if self.corner_radius != prev.corner_radius {
            widget::SizedBox::set_rounded(&mut element, self.corner_radius);
        }
        if self.padding != prev.padding {
            widget::SizedBox::set_padding(&mut element, self.padding);
        }
        {
            let mut child = widget::SizedBox::child_mut(&mut element)
                .expect("We only create SizedBox with a child");
            self.inner
                .rebuild(&prev.inner, view_state, ctx, child.downcast());
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<Self::Element>,
    ) {
        let mut child = widget::SizedBox::child_mut(&mut element)
            .expect("We only create SizedBox with a child");
        self.inner.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> crate::MessageResult<Action> {
        self.inner.message(view_state, id_path, message, app_state)
    }
}

/// Something that can be used as the border for a widget.
#[derive(PartialEq)]
struct BorderStyle {
    width: f64,
    brush: Brush,
}
