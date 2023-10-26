// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use peniko::Brush;

use crate::{
    class::Class,
    clicked::Clicked,
    common_attrs::{Fill, Stroke},
    pointer::{Pointer, PointerMsg},
    view::View,
};

pub trait ViewExt<T>: View<T> + Sized {
    fn clicked<F: Fn(&mut T)>(self, f: F) -> Clicked<T, Self, F> {
        crate::clicked::clicked(self, f)
    }

    fn pointer<F: Fn(&mut T, PointerMsg)>(self, f: F) -> Pointer<T, Self, F> {
        crate::pointer::pointer(self, f)
    }

    fn class(self, class: impl Into<String>) -> Class<T, Self> {
        crate::class::class(self, class)
    }

    fn fill(self, brush: impl Into<Brush>) -> Fill<T, Self> {
        crate::common_attrs::fill(self, brush)
    }

    fn stroke(self, brush: impl Into<Brush>, style: peniko::kurbo::Stroke) -> Stroke<T, Self> {
        crate::common_attrs::stroke(self, brush, style)
    }
}

impl<T, V: View<T>> ViewExt<T> for V {}
