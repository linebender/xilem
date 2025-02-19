// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use anymap3::Entry;

use crate::core::{FromDynWidget, MutateCtx, PropertiesMut, Widget, WidgetProperty};
use crate::kurbo::Affine;

// TODO - Document extension trait workaround.
// See https://xi.zulipchat.com/#narrow/stream/317477-masonry/topic/Thoughts.20on.20simplifying.20WidgetMut/near/436478885
/// A rich mutable reference to a [`Widget`].
///
/// In Masonry, widgets can't be mutated directly. All mutations go through a `WidgetMut`
/// wrapper. So, to change a label's text, you might call `WidgetMut<Label>::set_text()`.
/// This helps Masonry make sure that internal metadata is propagated after every widget
/// change.
///
/// You can create a `WidgetMut` from [`TestHarness`](crate::testing::TestHarness),
/// [`EventCtx`](crate::core::EventCtx), [`UpdateCtx`](crate::core::UpdateCtx) or from a parent
/// `WidgetMut` with [`MutateCtx`].
///
/// `WidgetMut` implements [`Deref`](std::ops::Deref) with `W::Mut` as target.
///
/// ## `WidgetMut` as a Receiver
///
/// Once the Receiver trait is stabilized, `WidgetMut` will implement it so that custom
/// widgets in downstream crates can use `WidgetMut` as the receiver for inherent methods.
pub struct WidgetMut<'a, W: Widget + ?Sized> {
    pub ctx: MutateCtx<'a>,
    pub properties: PropertiesMut<'a>,
    pub widget: &'a mut W,
}

impl<W: Widget + ?Sized> Drop for WidgetMut<'_, W> {
    fn drop(&mut self) {
        // If this `WidgetMut` is a reborrow, a parent non-reborrow `WidgetMut`
        // still exists which will do the merge-up in `Drop`.
        if let Some(parent_widget_state) = self.ctx.parent_widget_state.take() {
            parent_widget_state.merge_up(self.ctx.widget_state);
        }
    }
}

impl<W: Widget + ?Sized> WidgetMut<'_, W> {
    /// Get a `WidgetMut` for the same underlying widget with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> WidgetMut<'_, W> {
        let widget = &mut self.widget;
        WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            properties: self.properties.reborrow_mut(),
            widget,
        }
    }

    /// Returns true if the widget has a property of type `T`.
    pub fn get_prop<T: WidgetProperty>(&self) -> Option<&T> {
        self.properties.get::<T>()
    }

    /// Get value of property `T`, or None if the widget has no `T` property.
    pub fn contains_prop<T: WidgetProperty>(&self) -> bool {
        self.properties.contains::<T>()
    }

    /// Get value of property `T`, or None if the widget has no `T` property.
    pub fn get_prop_mut<T: WidgetProperty>(&mut self) -> Option<&mut T> {
        T::changed(&mut self.ctx);
        self.properties.get_mut::<T>()
    }

    /// Set property `T` to given value. Returns the previous value if `T` was already set.
    pub fn insert_prop<T: WidgetProperty>(&mut self, value: T) -> Option<T> {
        T::changed(&mut self.ctx);
        self.properties.insert(value)
    }

    /// Remove property `T`. Returns the previous value if `T` was set.
    pub fn remove_prop<T: WidgetProperty>(&mut self) -> Option<T> {
        T::changed(&mut self.ctx);
        self.properties.remove::<T>()
    }

    /// Returns an entry that can be used to add, update, or remove a property.
    pub fn prop_entry<T: WidgetProperty>(&mut self) -> Entry<'_, dyn std::any::Any, T> {
        T::changed(&mut self.ctx);
        self.properties.entry::<T>()
    }

    /// Set the local transform of this widget.
    ///
    /// It behaves similarly as CSS transforms.
    pub fn set_transform(&mut self, transform: Affine) {
        self.ctx.set_transform(transform);
    }

    /// Attempt to downcast to `WidgetMut` of concrete Widget type.
    pub fn try_downcast<W2: Widget + FromDynWidget + ?Sized>(
        &mut self,
    ) -> Option<WidgetMut<'_, W2>> {
        Some(WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            properties: self.properties.reborrow_mut(),
            widget: W2::from_dyn_mut(self.widget.as_mut_dyn())?,
        })
    }

    /// Downcasts to `WidgetMut` of concrete Widget type.
    ///
    /// ## Panics
    ///
    /// Panics if the downcast fails, with an error message that shows the
    /// discrepancy between the expected and actual types.
    pub fn downcast<W2: Widget + FromDynWidget + ?Sized>(&mut self) -> WidgetMut<'_, W2> {
        let w1_name = self.widget.type_name();
        match W2::from_dyn_mut(self.widget.as_mut_dyn()) {
            Some(widget) => WidgetMut {
                ctx: self.ctx.reborrow_mut(),
                properties: self.properties.reborrow_mut(),
                widget,
            },
            None => {
                panic!(
                    "failed to downcast widget: expected widget of type `{}`, found `{}`",
                    std::any::type_name::<W2>(),
                    w1_name,
                );
            }
        }
    }
}

// TODO - unit tests
