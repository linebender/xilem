// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use vello::kurbo::Affine;

use crate::{
    core::{FromDynWidget, MutateCtx, Property, Widget, WidgetId},
    properties::Dimensions,
};

/// A rich mutable reference to a [`Widget`].
///
/// In Masonry, widgets can't be mutated directly.
/// All mutations go through a `WidgetMut` wrapper.
/// So, to change a label's text, you might call `Label::set_text(WidgetMut<Label>)`.
/// This helps Masonry make sure that internal metadata is propagated after every widget
/// change.
///
/// You can create a `WidgetMut` from [`RenderRoot`](crate::app::RenderRoot),
/// [`EventCtx`](crate::core::EventCtx), [`UpdateCtx`](crate::core::UpdateCtx) or from a parent
/// `WidgetMut` with [`MutateCtx`].
///
/// # `WidgetMut` as a Receiver
///
/// Once the Receiver trait is stabilized, `WidgetMut` will implement it so that custom
/// widgets in downstream crates can use `WidgetMut` as the receiver for inherent methods.
#[non_exhaustive]
pub struct WidgetMut<'a, W: Widget + ?Sized> {
    /// The widget we're mutating.
    pub widget: &'a mut W,
    /// A context handle that points to the widget state and other relevant data.
    pub ctx: MutateCtx<'a>,
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
    /// Returns the [`WidgetId`] of the current widget.
    pub fn id(&self) -> WidgetId {
        self.ctx.widget_state.id
    }

    /// Returns a `WidgetMut` for the same underlying widget with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> WidgetMut<'_, W> {
        let widget = &mut self.widget;
        WidgetMut {
            widget,
            ctx: self.ctx.reborrow_mut(),
        }
    }

    /// Returns `true` if the widget has a local property of type `T`.
    ///
    /// Does not check default properties.
    pub fn contains_prop<T: Property>(&self) -> bool {
        self.ctx.properties.contains::<T>()
    }

    /// Whether the property `P` of this widget has been modified in this pass.
    ///
    /// This is useful when composing units which might mutate a property value.
    /// In these cases, the "strongest" of these will want to check if it has been
    /// changed by a different unit, to overwrite this change.
    pub fn prop_has_changed<P: Property>(&self) -> bool {
        self.ctx.changed_properties.contains(&TypeId::of::<P>())
    }

    /// Returns the value of property `T`.
    ///
    /// If the widget has an entry for `P`, returns that entry.
    /// If the default property set has an entry for `P`, returns that entry.
    /// Otherwise returns [`Property::static_default()`].
    pub fn get_prop<T: Property>(&self) -> &T {
        self.ctx.properties.get::<T>()
    }

    /// Sets property `T` to given value. Returns the previous value if `T` was already set locally.
    ///
    /// Does not affect default properties.
    ///
    /// This also calls [`Widget::property_changed`] with the matching type id.
    pub fn insert_prop<P: Property>(&mut self, value: P) -> Option<P> {
        self.ctx.changed_properties.insert(TypeId::of::<P>());
        let value = self.ctx.properties.insert(value);
        let mut ctx = self.ctx.update_mut();
        let property_type = TypeId::of::<P>();
        Dimensions::prop_changed(&mut ctx, property_type);
        self.widget.property_changed(&mut ctx, property_type);
        value
    }

    /// Removes property `T`. Returns the previous value if `T` was set locally.
    ///
    /// Does not affect default properties.
    ///
    /// This also calls [`Widget::property_changed`] with the matching type id.
    pub fn remove_prop<P: Property>(&mut self) -> Option<P> {
        self.ctx.changed_properties.insert(TypeId::of::<P>());
        let value = self.ctx.properties.remove::<P>();
        let mut ctx = self.ctx.update_mut();
        let property_type = TypeId::of::<P>();
        Dimensions::prop_changed(&mut ctx, property_type);
        self.widget.property_changed(&mut ctx, property_type);
        value
    }

    /// Sets the local transform of this widget.
    ///
    /// It behaves similarly as CSS transforms.
    pub fn set_transform(&mut self, transform: Affine) {
        self.ctx.set_transform(transform);
    }

    /// Attempts to downcast to `WidgetMut` of concrete widget type.
    pub fn try_downcast<W2: Widget + FromDynWidget + ?Sized>(
        &mut self,
    ) -> Option<WidgetMut<'_, W2>> {
        Some(WidgetMut {
            ctx: self.ctx.reborrow_mut(),
            widget: W2::from_dyn_mut(self.widget.as_mut_dyn())?,
        })
    }

    /// Downcasts to `WidgetMut` of concrete widget type.
    ///
    /// # Panics
    ///
    /// Panics if the downcast fails, with an error message that shows the
    /// discrepancy between the expected and actual types.
    pub fn downcast<W2: Widget + FromDynWidget + ?Sized>(&mut self) -> WidgetMut<'_, W2> {
        let w1_name = self.widget.type_name();
        match W2::from_dyn_mut(self.widget.as_mut_dyn()) {
            Some(widget) => WidgetMut {
                widget,
                ctx: self.ctx.reborrow_mut(),
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
