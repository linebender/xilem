// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Traits used to set custom styles on views.

use masonry::core::Property;
use masonry::properties::types::Gradient;
use masonry::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, BoxShadow, CornerRadius,
    DisabledBackground, HoveredBorderColor, Padding,
};
use vello::peniko::Color;

/// Trait implemented by views to signal that a given property can be set on them.
///
/// In most cases, you should implement this trait through [`declare_property_tuple!`](crate::declare_property_tuple)
/// when authoring views.
pub trait HasProperty<P: Property>: Style {
    /// Return a mutable reference to the specific property.
    fn property(&mut self) -> &mut Option<P>;
}

/// Trait implemented by most views that lets you set some styling properties on them.
///
/// Which methods you can use will depend on which parameter the element implements [`HasProperty`] with,
/// which matches which [`Properties`](masonry::core::Properties) the underlying widget handles.
pub trait Style: Sized {
    /// The tuple type used by the element to store properties.
    type Props;

    /// Return a mutable reference to the element's property storage.
    fn properties(&mut self) -> &mut Self::Props;

    /// Set the element's background to a color/gradient.
    fn background(mut self, background: Background) -> Self
    where
        Self: HasProperty<Background>,
    {
        *self.property() = Some(background);
        self
    }

    /// Set the element's background to a color.
    fn background_color(mut self, color: Color) -> Self
    where
        Self: HasProperty<Background>,
    {
        *self.property() = Some(Background::Color(color));
        self
    }

    /// Set the element's background to a gradient.
    fn background_gradient(mut self, gradient: Gradient) -> Self
    where
        Self: HasProperty<Background>,
    {
        *self.property() = Some(Background::Gradient(gradient));
        self
    }

    /// Set the element's background when pressed to a color/gradient.
    fn active_background(mut self, background: Background) -> Self
    where
        Self: HasProperty<ActiveBackground>,
    {
        *self.property() = Some(ActiveBackground(background));
        self
    }

    /// Set the element's background when pressed to a color.
    fn active_background_color(mut self, color: Color) -> Self
    where
        Self: HasProperty<ActiveBackground>,
    {
        *self.property() = Some(ActiveBackground(Background::Color(color)));
        self
    }

    /// Set the element's background when pressed to a gradient.
    fn active_background_gradient(mut self, gradient: Gradient) -> Self
    where
        Self: HasProperty<ActiveBackground>,
    {
        *self.property() = Some(ActiveBackground(Background::Gradient(gradient)));
        self
    }

    /// Set the element's background when disabled to a color/gradient.
    fn disabled_background(mut self, background: Background) -> Self
    where
        Self: HasProperty<DisabledBackground>,
    {
        *self.property() = Some(DisabledBackground(background));
        self
    }

    /// Set the element's background when disabled to a color.
    fn disabled_background_color(mut self, color: Color) -> Self
    where
        Self: HasProperty<DisabledBackground>,
    {
        *self.property() = Some(DisabledBackground(Background::Color(color)));
        self
    }

    /// Set the element's background when disabled to a gradient.
    fn disabled_background_gradient(mut self, gradient: Gradient) -> Self
    where
        Self: HasProperty<DisabledBackground>,
    {
        *self.property() = Some(DisabledBackground(Background::Gradient(gradient)));
        self
    }

    /// Set the element's border color and width.
    fn border(mut self, color: Color, width: f64) -> Self
    where
        Self: HasProperty<BorderColor>,
        Self: HasProperty<BorderWidth>,
    {
        *self.property() = Some(BorderColor { color });
        *self.property() = Some(BorderWidth { width });
        self
    }

    /// Set the element's border color.
    fn border_color(mut self, color: Color) -> Self
    where
        Self: HasProperty<BorderColor>,
    {
        *self.property() = Some(BorderColor { color });
        self
    }

    /// Set the element's border color when hovered.
    fn hovered_border_color(mut self, color: Color) -> Self
    where
        Self: HasProperty<HoveredBorderColor>,
    {
        *self.property() = Some(HoveredBorderColor(BorderColor { color }));
        self
    }

    /// Set the element's border width.
    fn border_width(mut self, width: f64) -> Self
    where
        Self: HasProperty<BorderWidth>,
    {
        *self.property() = Some(BorderWidth { width });
        self
    }

    /// Set the element's box shadow.
    fn box_shadow(mut self, box_shadow: BoxShadow) -> Self
    where
        Self: HasProperty<BoxShadow>,
    {
        *self.property() = Some(box_shadow);
        self
    }

    /// Set the element's corner radius.
    fn corner_radius(mut self, radius: f64) -> Self
    where
        Self: HasProperty<CornerRadius>,
    {
        *self.property() = Some(CornerRadius { radius });
        self
    }

    /// Set the element's padding.
    fn padding(mut self, padding: impl Into<Padding>) -> Self
    where
        Self: HasProperty<Padding>,
    {
        *self.property() = Some(padding.into());
        self
    }
}
