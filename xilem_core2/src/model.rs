// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Model version of Masonry for exploration

use core::any::Any;

use alloc::boxed::Box;

use crate::{Element, SuperElement, View};

pub trait Widget: 'static + Any {
    fn as_mut_any(&mut self) -> &mut dyn Any;
}
pub struct WidgetPod<W: Widget> {
    widget: W,
}
pub struct WidgetMut<'a, W: Widget> {
    value: &'a mut W,
}
impl Widget for Box<dyn Widget> {
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

// Model version of xilem_masonry (`xilem`)

// Hmm, this implementation can't exist in `xilem` if `xilem_core` is a different crate
// due to the orphan rules...
impl<W: Widget> Element for WidgetPod<W> {
    type Mut<'a> = WidgetMut<'a, W>;

    /// This implementation will perform `merge_up` multiple times, but that's
    /// already true for downcasting anyway, so merge_up is already idempotent
    fn with_reborrow_val<'o, R: 'static>(
        this: Self::Mut<'o>,
        f: impl FnOnce(Self::Mut<'_>) -> R,
    ) -> (Self::Mut<'o>, R) {
        let value = WidgetMut { value: this.value };
        let ret = f(value);
        (this, ret)
    }
}

impl View<(), ()> for Button {
    type Element = WidgetPod<ButtonWidget>;
    type ViewState = ();
}

pub struct Button {}

pub struct ButtonWidget {}
impl Widget for ButtonWidget {
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<W: Widget> SuperElement<WidgetPod<W>> for WidgetPod<Box<dyn Widget>> {
    fn upcast(child: WidgetPod<W>) -> Self {
        WidgetPod {
            widget: Box::new(child.widget),
        }
    }
    fn with_downcast_val<'a, R>(
        this: Self::Mut<'a>,
        f: impl FnOnce(<WidgetPod<W> as Element>::Mut<'_>) -> R,
    ) -> (Self::Mut<'a>, R) {
        let value = WidgetMut {
        value: this.value.as_mut_any().downcast_mut().expect(
            "this widget should have been created from a child widget of type `W` in `Self::upcast`",
        ),
    };
        let ret = f(value);
        (this, ret)
    }
}
