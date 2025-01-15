// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::Affine;
use xilem_core::{DynMessage, View, ViewMarker};

use crate::{Pod, ViewCtx, WidgetView};

/// An extension trait, to allow common transformations of the views transform.
pub trait Transformable: Sized {
    fn transform_mut(&mut self) -> &mut Affine;

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
    fn transform(mut self, v: impl Into<Affine>) -> Self {
        *self.transform_mut() *= v.into();
        self
    }
}
