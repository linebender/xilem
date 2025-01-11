// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Views for the widgets which are built-in to Masonry. These are the primitives your Xilem app's view tree will generally be constructed from.

mod task;
pub use task::*;

mod worker;
pub use worker::*;

mod button;
pub use button::*;

mod checkbox;
pub use checkbox::*;

mod flex;
pub use flex::*;

mod grid;
pub use grid::*;

mod sized_box;
pub use sized_box::*;

mod spinner;
pub use spinner::*;

mod image;
pub use image::*;

mod label;
pub use label::*;

mod variable_label;
pub use variable_label::*;

mod progress_bar;
pub use progress_bar::*;

mod prose;
pub use prose::*;

mod textbox;
pub use textbox::*;

mod portal;
pub use portal::*;

mod zstack;
pub use zstack::*;

/// An extension trait, to allow common transformations of the views transform.
pub trait Transformable: Sized {
    fn transform_mut(&mut self) -> &mut crate::Affine;

    #[must_use]
    fn rotate(mut self, radians: f64) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_rotate(radians);
        self
    }

    #[must_use]
    fn scale(mut self, uniform: f64) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_scale(uniform);
        self
    }

    #[must_use]
    fn scale_non_uniform(mut self, x: f64, y: f64) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_scale_non_uniform(x, y);
        self
    }

    #[must_use]
    fn translate(mut self, v: impl Into<crate::Vec2>) -> Self {
        let transform = self.transform_mut();
        *transform = transform.then_translate(v.into());
        self
    }

    #[must_use]
    fn transform(mut self, v: impl Into<crate::Affine>) -> Self {
        *self.transform_mut() *= v.into();
        self
    }
}
