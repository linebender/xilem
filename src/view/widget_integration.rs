// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use xilem_core::{GenericView, TraitBound, TraitPod};

use crate::widget::AnyWidget;

pub type WidgetBound = dyn AnyWidget + 'static;

pub trait View<T, A = ()>: GenericView<T, WidgetBound, A> {}

impl<T, A, V: GenericView<T, WidgetBound, A>> View<T, A> for V {}

impl TraitPod for WidgetBound {
    type Pod = crate::widget::Pod;

    fn make_pod(w: Box<Self>) -> Self::Pod {
        crate::widget::Pod::new_from_box(w)
    }
}

// The following implementation would be nice but seems impossible
// due to coherency.

/*
impl<W: Widget + 'static> TraitBound<WidgetBound> for W {
    fn boxed(self) -> Box<dyn AnyWidget + 'static> {
        Box::new(self)
    }

    fn as_mut(&mut self) -> &mut (dyn AnyWidget + 'static) {
        self
    }
}
*/

macro_rules! impl_trait_bound {
    ($widget:ty) => {
        impl TraitBound<WidgetBound> for $widget {
            fn boxed(self) -> Box<dyn AnyWidget + 'static> {
                Box::new(self)
            }

            fn as_mut(&mut self) -> &mut (dyn AnyWidget + 'static) {
                self
            }
        }
    };
}

// Arguably these should move to the same place as the corresponding `impl Widget`
impl_trait_bound!(crate::widget::Button);
impl_trait_bound!(crate::widget::LinearLayout);
impl_trait_bound!(Box<dyn AnyWidget>);
