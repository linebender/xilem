// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Traits used to set custom styles on views.

use masonry::core::HasProperty;
use masonry::properties::{ContentColor, DisabledContentColor, FocusedBorderColor, LineBreaking};
use vello::peniko::Color;

pub use masonry::properties::types::{Gradient, GradientShape};
pub use masonry::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, BoxShadow, CornerRadius,
    DisabledBackground, HoveredBorderColor, Padding,
};

use crate::WidgetView;
use crate::core::ViewArgument;
use crate::view::Prop;

/// Trait implemented by most widget views that lets you style their properties.
///
/// Which methods you can use will depend whether the underlying widget implements [`HasProperty`].
pub trait Style<State: ViewArgument, Action: 'static>: WidgetView<State, Action> + Sized {
    /// Set the element's content color.
    ///
    /// "Content color" usually means text or text decorations.
    fn color(self, color: Color) -> Prop<ContentColor, Self, State, Action>
    where
        Self::Widget: HasProperty<ContentColor>,
    {
        self.prop(ContentColor { color })
    }

    /// Set the element's content color when disabled.
    ///
    /// "Content color" usually means text or text decorations.
    fn disabled_color(self, color: Color) -> Prop<DisabledContentColor, Self, State, Action>
    where
        Self::Widget: HasProperty<DisabledContentColor>,
    {
        self.prop(DisabledContentColor(ContentColor { color }))
    }

    /// Set the element's background to a color/gradient.
    fn background(self, background: Background) -> Prop<Background, Self, State, Action>
    where
        Self::Widget: HasProperty<Background>,
    {
        self.prop(background)
    }

    /// Set the element's background to a color.
    fn background_color(self, color: Color) -> Prop<Background, Self, State, Action>
    where
        Self::Widget: HasProperty<Background>,
    {
        self.prop(Background::Color(color))
    }

    /// Set the element's background to a gradient.
    fn background_gradient(self, gradient: Gradient) -> Prop<Background, Self, State, Action>
    where
        Self::Widget: HasProperty<Background>,
    {
        self.prop(Background::Gradient(gradient))
    }

    /// Set the element's background when pressed to a color/gradient.
    fn active_background(
        self,
        background: Background,
    ) -> Prop<ActiveBackground, Self, State, Action>
    where
        Self::Widget: HasProperty<ActiveBackground>,
    {
        self.prop(ActiveBackground(background))
    }

    /// Set the element's background when pressed to a color.
    fn active_background_color(self, color: Color) -> Prop<ActiveBackground, Self, State, Action>
    where
        Self::Widget: HasProperty<ActiveBackground>,
    {
        self.prop(ActiveBackground(Background::Color(color)))
    }

    /// Set the element's background when pressed to a gradient.
    fn active_background_gradient(
        self,
        gradient: Gradient,
    ) -> Prop<ActiveBackground, Self, State, Action>
    where
        Self::Widget: HasProperty<ActiveBackground>,
    {
        self.prop(ActiveBackground(Background::Gradient(gradient)))
    }

    /// Set the element's background when disabled to a color/gradient.
    fn disabled_background(
        self,
        background: Background,
    ) -> Prop<DisabledBackground, Self, State, Action>
    where
        Self::Widget: HasProperty<DisabledBackground>,
    {
        self.prop(DisabledBackground(background))
    }

    /// Set the element's background when disabled to a color.
    fn disabled_background_color(
        self,
        color: Color,
    ) -> Prop<DisabledBackground, Self, State, Action>
    where
        Self::Widget: HasProperty<DisabledBackground>,
    {
        self.prop(DisabledBackground(Background::Color(color)))
    }

    /// Set the element's background when disabled to a gradient.
    fn disabled_background_gradient(
        self,
        gradient: Gradient,
    ) -> Prop<DisabledBackground, Self, State, Action>
    where
        Self::Widget: HasProperty<DisabledBackground>,
    {
        self.prop(DisabledBackground(Background::Gradient(gradient)))
    }

    /// Set the element's border color and width.
    fn border(
        self,
        color: Color,
        width: f64,
    ) -> Prop<BorderWidth, Prop<BorderColor, Self, State, Action>, State, Action>
    where
        Self::Widget: HasProperty<BorderColor> + HasProperty<BorderWidth>,
    {
        self.prop(BorderColor { color }).prop(BorderWidth { width })
    }

    /// Set the element's border color.
    fn border_color(self, color: Color) -> Prop<BorderColor, Self, State, Action>
    where
        Self::Widget: HasProperty<BorderColor>,
    {
        self.prop(BorderColor { color })
    }

    /// Set the element's border color when hovered.
    fn hovered_border_color(self, color: Color) -> Prop<HoveredBorderColor, Self, State, Action>
    where
        Self::Widget: HasProperty<HoveredBorderColor>,
    {
        self.prop(HoveredBorderColor(BorderColor { color }))
    }

    /// Set the element's border color when focused.
    fn focused_border_color(self, color: Color) -> Prop<FocusedBorderColor, Self, State, Action>
    where
        Self::Widget: HasProperty<FocusedBorderColor>,
    {
        self.prop(FocusedBorderColor(BorderColor { color }))
    }

    /// Set the element's border width.
    fn border_width(self, width: f64) -> Prop<BorderWidth, Self, State, Action>
    where
        Self::Widget: HasProperty<BorderWidth>,
    {
        self.prop(BorderWidth { width })
    }

    /// Set the element's box shadow.
    fn box_shadow(self, box_shadow: BoxShadow) -> Prop<BoxShadow, Self, State, Action>
    where
        Self::Widget: HasProperty<BoxShadow>,
    {
        self.prop(box_shadow)
    }

    /// Set the element's corner radius.
    fn corner_radius(self, radius: f64) -> Prop<CornerRadius, Self, State, Action>
    where
        Self::Widget: HasProperty<CornerRadius>,
    {
        self.prop(CornerRadius { radius })
    }

    /// Set the element's padding.
    fn padding(self, padding: impl Into<Padding>) -> Prop<Padding, Self, State, Action>
    where
        Self::Widget: HasProperty<Padding>,
    {
        self.prop(padding.into())
    }

    /// Set how line breaks will be handled when text overflows the available space.
    fn line_break_mode(
        self,
        line_break_mode: LineBreaking,
    ) -> Prop<LineBreaking, Self, State, Action>
    where
        Self::Widget: HasProperty<LineBreaking>,
    {
        self.prop(line_break_mode)
    }
}

impl<State, Action, V> Style<State, Action> for V
where
    State: ViewArgument,
    Action: 'static,
    V: WidgetView<State, Action> + Sized,
{
}
