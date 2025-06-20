// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

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
    fn rebuild_properties(&self, prev: &Self, target: &mut WidgetMut<'_, impl Widget>);
}

impl<P0: Property + Eq + Clone> PropertyTuple for (Option<P0>,) {
    fn build_properties(&self) -> Properties {
        let mut props = Properties::new();
        if let Some(prop) = self.0.clone() {
            props.insert(prop);
        }
        props
    }

    fn rebuild_properties(&self, prev: &Self, target: &mut WidgetMut<'_, impl Widget>) {
        if self.0 != prev.0 {
            if let Some(prop) = self.0.clone() {
                target.insert_prop(prop);
            } else {
                target.remove_prop::<P0>();
            }
        }
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

            fn rebuild_properties(&self, prev: &Self, target: &mut WidgetMut<'_, impl Widget>) {
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

        }

    };
}

// The (P0,) one-view tuple case is covered outside the macro,
// for easier code editing.

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

// ---

// We expect to use the ${index} metavariable here once it's stable
// https://veykril.github.io/tlborm/decl-macros/minutiae/metavar-expr.html

/// Macro used to declare a list of properties for a view.
///
/// The expected way to invoke this macro is:
///
/// ```ignore
/// declare_property_tuple!(
///     MyWidgetProps;
///     MyView;
///
///     SomeProp, 0;
///     OtherProp, 1;
///     AnotherProp, 2;
///     YetOtherProp, 3;
///     // ...
/// );
/// ```
///
/// This will declare a type `MyWidgetProp` as an alias to `(Option<SomeProp>, Option<OtherProp>, ...)`.
///
/// This will also implement [`HasProperty<Prop>`](crate::style::HasProperty) for `MyView` with each of the listed properties.
/// Doing so enables using the corresponding extension method for each of them in the [`Style`](crate::style::Style) trait.
///
/// If `MyView` is a generic type with type params `T`, `U`, `V`, you should invoke the macro like this:
///
/// ```ignore
/// declare_property_tuple!(
///     MyWidgetProps;
///     MyView<T, U, V>;
///
///     SomeProp, 0;
///     OtherProp, 1;
///     AnotherProp, 2;
///     YetOtherProp, 3;
///     // ...
/// );
/// ```
#[macro_export]
macro_rules! declare_property_tuple {
    (
        $Props: ident ;
        $Self:ident $( < $($Args:ident),* > )? ;

        $(
            $Type: ident, $idx: tt;
        )+
    ) => {
        $crate::__declare_property_tuple_loop!(
            $Props; ( $Self $( <$($Args),*> )? ); $($Type, $idx;)+
        );
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __declare_property_tuple_loop {
    (
        $Props: ident ;
        $Self:tt;

        $(
            $Type: ident, $idx: tt;
        )+
    ) => {
        type $Props = ($(Option<$Type>,)+);

        $(
            $crate::__declare_property_tuple_inner!($Self; $Type; $idx;);
        )+

    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __declare_property_tuple_inner {
    (
        (
            $Self:ident $( < $($Args:ident),* > )?
        );
        $Type: ident; $idx: tt;
    )
    =>
    {
        impl $( <$($Args,)*> )? $crate::style::HasProperty<$Type> for $Self $( <$($Args),*> )? {
            fn property(&mut self) -> &mut Option<$Type> {
                    &mut self.properties().$idx
            }
        }
    };
}
