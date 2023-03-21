// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    class::Class,
    clicked::Clicked,
    pointer::{Pointer, PointerMsg},
    view::View,
};

pub trait ViewExt<T>: View<T> + Sized {
    fn clicked<F: Fn(&mut T)>(self, f: F) -> Clicked<Self, F>;
    fn pointer<F: Fn(&mut T, PointerMsg)>(self, f: F) -> Pointer<Self, F> {
        crate::pointer::pointer(self, f)
    }
    fn class(self, class: impl Into<String>) -> Class<Self> {
        crate::class::class(self, class)
    }
}

impl<T, V: View<T>> ViewExt<T> for V {
    fn clicked<F: Fn(&mut T)>(self, f: F) -> Clicked<Self, F> {
        crate::clicked::clicked(self, f)
    }
}
