use std::{borrow::Cow, marker::PhantomData};

use xilem_core::{Id, MessageResult};

use crate::{
    interfaces::{sealed::Sealed, Element},
    ChangeFlags, Cx, View, ViewMarker,
};

/// A trait to make the class adding functions generic over collection type
pub trait IntoClasses {
    fn into_classes(self, classes: &mut Vec<Cow<'static, str>>);
}

impl IntoClasses for String {
    fn into_classes(self, classes: &mut Vec<Cow<'static, str>>) {
        classes.push(self.into());
    }
}

impl IntoClasses for &'static str {
    fn into_classes(self, classes: &mut Vec<Cow<'static, str>>) {
        classes.push(self.into())
    }
}

impl<T> IntoClasses for Option<T>
where
    T: IntoClasses,
{
    fn into_classes(self, classes: &mut Vec<Cow<'static, str>>) {
        if let Some(t) = self {
            t.into_classes(classes)
        }
    }
}

impl<T> IntoClasses for Vec<T>
where
    T: IntoClasses,
{
    fn into_classes(self, classes: &mut Vec<Cow<'static, str>>) {
        for itm in self {
            itm.into_classes(classes);
        }
    }
}

macro_rules! impl_tuple_intoclasses {
    ($($name:ident : $type:ident),* $(,)?) => {
        impl<$($type),*> IntoClasses for ($($type,)*)
        where
            $($type: IntoClasses),*
        {
            #[allow(unused_variables)]
            fn into_classes(self, classes: &mut Vec<Cow<'static, str>>) {
                let ($($name,)*) = self;
                $(
                    $name.into_classes(classes);
                )*
            }
        }
    };
}

impl_tuple_intoclasses!();
impl_tuple_intoclasses!(t1: T1);
impl_tuple_intoclasses!(t1: T1, t2: T2);
impl_tuple_intoclasses!(t1: T1, t2: T2, t3: T3);
impl_tuple_intoclasses!(t1: T1, t2: T2, t3: T3, t4: T4);

/// Applies a class to the underlying element.
pub struct Class<E, T, A> {
    pub(crate) element: E,
    pub(crate) class_names: Vec<Cow<'static, str>>,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, T, A> ViewMarker for Class<E, T, A> {}
impl<E, T, A> Sealed for Class<E, T, A> {}

impl<E: Element<T, A>, T, A> View<T, A> for Class<E, T, A> {
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        for class_name in &self.class_names {
            cx.add_class_to_element(class_name);
        }
        self.element.build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        for class_name in &self.class_names {
            cx.add_class_to_element(class_name);
        }
        self.element.rebuild(cx, &prev.element, id, state, element)
    }

    fn message(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> MessageResult<A> {
        self.element.message(id_path, state, message, app_state)
    }
}

crate::interfaces::impl_dom_interfaces_for_ty!(Element, Class);
