// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::{Any, TypeId, type_name};

use masonry::core::{Properties, Property, Widget, WidgetMut};

/// Helper trait implemented for all tuples of `Option<SomeProperty>` up to 12 items.
pub trait PropertyTuple {
    /// Helper method for [`xilem_core::View::build`].
    ///
    /// Returns list of properties to be set on the created widget.
    fn build_properties(&self) -> Properties;

    /// Helper method for [`xilem_core::View::rebuild`].
    ///
    /// Check if any property has changed, and if so, applies it to the given widget.
    fn rebuild_properties(&self, prev: &Self, target: &mut WidgetMut<impl Widget>);

    /// Returns a mutable reference to the first tuple item of type `P`.
    ///
    /// ## Panic
    ///
    /// Panics if `Self` does not have an item fo type `P`.
    fn property_mut<P: Property>(&mut self) -> &mut Option<P>;
}

impl<P0: Property + Eq + Clone> PropertyTuple for (Option<P0>,) {
    fn build_properties(&self) -> Properties {
        let mut props = Properties::new();
        if let Some(prop) = self.0.clone() {
            props.insert(prop);
        }
        props
    }

    fn rebuild_properties(&self, prev: &Self, target: &mut WidgetMut<impl Widget>) {
        if self.0 != prev.0 {
            if let Some(prop) = self.0.clone() {
                target.insert_prop(prop);
            } else {
                target.remove_prop::<P0>();
            }
        }
    }

    fn property_mut<P: Property>(&mut self) -> &mut Option<P> {
        let mut prop = None;
        for elem in [&mut self.0 as &mut dyn Any] {
            if TypeId::of::<Option<P>>() == (*elem).type_id() {
                prop = Some(elem);
                break;
            }
        }
        let Some(prop) = prop else {
            panic!("Property '{}' not found.", type_name::<P>());
        };
        prop.downcast_mut().unwrap()
    }
}

// We expect to use the ${index} metavariable here once it's stable
// https://veykril.github.io/tlborm/decl-macros/minutiae/metavar-expr.html
macro_rules! impl_property_tuple {
    ($($Type: ident, $idx: tt);+) => {

        impl<$($Type,)+> PropertyTuple for ($(Option<$Type>,)+)
            where $($Type: Property + PartialEq + Clone,)+
        {
            fn build_properties(&self) -> Properties {
                let mut props = Properties::new();
                $(
                    if let Some(prop) = self.$idx.clone() {
                        props.insert(prop);
                    }
                )+
                props
            }

            fn rebuild_properties(&self, prev: &Self, target: &mut WidgetMut<impl Widget>) {
                $(
                    if self.$idx != prev.$idx {
                        if let Some(prop) = self.$idx.clone() {
                            target.insert_prop(prop);
                        } else {
                            target.remove_prop::<$Type>();
                        }
                    }
                )+
            }

            fn property_mut<P: Property>(&mut self) -> &mut Option<P> {
                let mut prop = None;
                for elem in [$( &mut self.$idx as &mut dyn Any, )+] {
                    if TypeId::of::<Option<P>>() == (*elem).type_id() {
                        prop = Some(elem);
                        break;
                    }
                }
                let Some(prop) = prop else {
                    panic!("Property '{}' not found.", type_name::<P>());
                };
                prop.downcast_mut().unwrap()
            }

        }

    };
}

// impl_property_tuple!(P0, 0);
impl_property_tuple!(P0, 0; P1, 1);
impl_property_tuple!(P0, 0; P1, 1; P2, 2);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8; P9, 9);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8; P9, 9; P10, 10);
impl_property_tuple!(P0, 0; P1, 1; P2, 2; P3, 3; P4, 4; P5, 5; P6, 6; P7, 7; P8, 8; P9, 9; P10, 10; P11, 11);
