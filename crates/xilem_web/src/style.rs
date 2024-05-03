// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::{borrow::Cow, collections::HashMap};

use xilem_core::{Id, MessageResult};

use crate::{interfaces::sealed::Sealed, ChangeFlags, Cx, View, ViewMarker};

use super::interfaces::Element;

/// A trait to make the class adding functions generic over collection type
pub trait IntoStyles {
    fn into_styles(self, styles: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>);
}

struct StyleTuple<T1, T2>(T1, T2);

/// Create a style from a style name and its value.
pub fn style<T1, T2>(name: T1, value: T2) -> impl IntoStyles
where
    T1: Into<Cow<'static, str>>,
    T2: Into<Cow<'static, str>>,
{
    StyleTuple(name, value)
}

impl<T1, T2> IntoStyles for StyleTuple<T1, T2>
where
    T1: Into<Cow<'static, str>>,
    T2: Into<Cow<'static, str>>,
{
    fn into_styles(self, styles: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>) {
        let StyleTuple(key, value) = self;
        styles.push((key.into(), value.into()));
    }
}

impl<T> IntoStyles for Option<T>
where
    T: IntoStyles,
{
    fn into_styles(self, styles: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>) {
        if let Some(t) = self {
            t.into_styles(styles);
        }
    }
}

impl<T> IntoStyles for Vec<T>
where
    T: IntoStyles,
{
    fn into_styles(self, styles: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>) {
        for itm in self {
            itm.into_styles(styles);
        }
    }
}

impl<T1, T2, S> IntoStyles for HashMap<T1, T2, S>
where
    T1: Into<Cow<'static, str>>,
    T2: Into<Cow<'static, str>>,
{
    fn into_styles(self, styles: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>) {
        for (key, value) in self {
            styles.push((key.into(), value.into()));
        }
    }
}

impl<T1, T2> IntoStyles for BTreeMap<T1, T2>
where
    T1: Into<Cow<'static, str>>,
    T2: Into<Cow<'static, str>>,
{
    fn into_styles(self, styles: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>) {
        for (key, value) in self {
            styles.push((key.into(), value.into()));
        }
    }
}

macro_rules! impl_tuple_intostyles {
    ($($name:ident : $type:ident),* $(,)?) => {
        impl<$($type),*> IntoStyles for ($($type,)*)
        where
            $($type: IntoStyles),*
        {
            #[allow(unused_variables)]
            fn into_styles(self, styles: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>) {
                let ($($name,)*) = self;
                $(
                    $name.into_styles(styles);
                )*
            }
        }
    };
}

impl_tuple_intostyles!();
impl_tuple_intostyles!(t1: T1);
impl_tuple_intostyles!(t1: T1, t2: T2);
impl_tuple_intostyles!(t1: T1, t2: T2, t3: T3);
impl_tuple_intostyles!(t1: T1, t2: T2, t3: T3, t4: T4);

pub struct Style<E, T, A> {
    pub(crate) element: E,
    pub(crate) styles: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    pub(crate) phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, T, A> ViewMarker for Style<E, T, A> {}
impl<E, T, A> Sealed for Style<E, T, A> {}

impl<E: Element<T, A>, T, A> View<T, A> for Style<E, T, A> {
    type State = E::State;
    type Element = E::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        for (key, value) in &self.styles {
            cx.add_style_to_element(key, value);
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
        for (key, value) in &self.styles {
            cx.add_style_to_element(key, value);
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

crate::interfaces::impl_dom_interfaces_for_ty!(Element, Style);
