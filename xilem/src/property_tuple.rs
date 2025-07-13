// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{Properties, Widget, WidgetMut};

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

        #[expect(missing_docs)]
        pub struct $Props($(Option<$Type>,)+);

        impl ::std::default::Default for $Props {
            fn default() -> Self {
                $Props {
                    $(
                        $idx: ::std::default::Default::default(),
                    )+
                }
            }
        }

        impl $crate::property_tuple::PropertyTuple for $Props
        {
            fn build_properties(&self) -> $crate::masonry::core::Properties {
                let mut props = $crate::masonry::core::Properties::new();
                $(
                    if let Some(prop) = self.$idx.clone() {
                        props.insert(prop);
                    }
                )+
                props
            }

            fn rebuild_properties(&self, prev: &Self, target: &mut $crate::masonry::core::WidgetMut<'_, impl $crate::masonry::core::Widget>) {
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

// Example impl for code editing
#[cfg(false)]
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
