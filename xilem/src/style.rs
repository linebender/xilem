// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Traits used to set custom syles on elements.

use masonry::core::Property;
use masonry::properties::types::Gradient;
use masonry::properties::{Background, BorderColor, BorderWidth, BoxShadow, CornerRadius, Padding};
use vello::peniko::Color;

use crate::property_tuple::PropertyTuple;

/// Marker trait implement by elements to signal that a given property can be set on them.
pub trait HasProperty<P: Property> {}

/// Trait implemented by most elements that lets you set some styling properties.
///
/// Which methods you can use will depend on which marker traits the element implements,
/// which matches which [`Properties`] the underlying widget handles.
pub trait Style: Sized {
    /// The tuple type used by the element to store properties.
    type Props: PropertyTuple;

    /// Return a mutable reference to the element's property storage.
    fn properties(&mut self) -> &mut Self::Props;

    /// Set the element's background color/gradient.
    fn background(mut self, background: Background) -> Self
    where
        Self: HasProperty<Background>,
    {
        *self.properties().property_mut() = Some(background);
        self
    }

    /// Set the element's background color.
    fn background_color(mut self, color: Color) -> Self
    where
        Self: HasProperty<Background>,
    {
        *self.properties().property_mut() = Some(Background::Color(color));
        self
    }

    /// Set the element's background gradient.
    fn background_gradient(mut self, gradient: Gradient) -> Self
    where
        Self: HasProperty<Background>,
    {
        *self.properties().property_mut() = Some(Background::Gradient(gradient));
        self
    }

    /// Set the element's border color and width.
    fn border(mut self, color: Color, width: f64) -> Self
    where
        Self: HasProperty<BorderColor>,
        Self: HasProperty<BorderWidth>,
    {
        *self.properties().property_mut() = Some(BorderColor { color });
        *self.properties().property_mut() = Some(BorderWidth { width });
        self
    }

    /// Set the element's border color.
    fn border_color(mut self, color: Color) -> Self
    where
        Self: HasProperty<BorderColor>,
    {
        *self.properties().property_mut() = Some(BorderColor { color });
        self
    }

    /// Set the element's border width.
    fn border_width(mut self, width: f64) -> Self
    where
        Self: HasProperty<BorderWidth>,
    {
        *self.properties().property_mut() = Some(BorderWidth { width });
        self
    }

    /// Set the element's box shadow.
    fn box_shadow(mut self, box_shadow: BoxShadow) -> Self
    where
        Self: HasProperty<BoxShadow>,
    {
        *self.properties().property_mut() = Some(box_shadow);
        self
    }

    /// Set the element's corner radius.
    fn corner_radius(mut self, radius: f64) -> Self
    where
        Self: HasProperty<CornerRadius>,
    {
        *self.properties().property_mut() = Some(CornerRadius { radius });
        self
    }

    /// Set the element's padding.
    fn padding(mut self, padding: impl Into<Padding>) -> Self
    where
        Self: HasProperty<Padding>,
    {
        *self.properties().property_mut() = Some(padding.into());
        self
    }
}
