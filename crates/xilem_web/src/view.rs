// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Integration with xilem_core. This instantiates the View and related
//! traits for DOM node generation.

use std::{any::Any, borrow::Cow, ops::Deref};

use xilem_core::{Id, MessageResult};

use crate::{context::Cx, ChangeFlags};

pub(crate) mod sealed {
    pub trait Sealed {}
}

// A possible refinement of xilem_core is to allow a single concrete type
// for a view element, rather than an associated type with a bound.
/// This trait is implemented for types that implement `AsRef<web_sys::Node>`.
/// It is an implementation detail.
pub trait DomNode: sealed::Sealed + 'static {
    fn into_pod(self) -> Pod;
    fn as_node_ref(&self) -> &web_sys::Node;
}

impl<N: AsRef<web_sys::Node> + 'static> sealed::Sealed for N {}
impl<N: AsRef<web_sys::Node> + 'static> DomNode for N {
    fn into_pod(self) -> Pod {
        Pod(Box::new(self))
    }

    fn as_node_ref(&self) -> &web_sys::Node {
        self.as_ref()
    }
}

/// A trait for types that can be type-erased and impl `AsRef<Node>`. It is an
/// implementation detail.
pub trait AnyNode: sealed::Sealed {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn as_node_ref(&self) -> &web_sys::Node;
}

impl<N: AsRef<web_sys::Node> + Any> AnyNode for N {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_node_ref(&self) -> &web_sys::Node {
        self.as_ref()
    }
}

impl sealed::Sealed for Box<dyn AnyNode> {}
impl DomNode for Box<dyn AnyNode> {
    fn into_pod(self) -> Pod {
        Pod(self)
    }

    fn as_node_ref(&self) -> &web_sys::Node {
        self.deref().as_node_ref()
    }
}

/// A container that holds a DOM element.
///
/// This implementation may be overkill (it's possibly enough that everything is
/// just a `web_sys::Element`), but does allow element types that contain other
/// data, if needed.
pub struct Pod(pub Box<dyn AnyNode>);

impl Pod {
    fn new(node: impl DomNode) -> Self {
        node.into_pod()
    }

    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.0.as_any_mut().downcast_mut()
    }

    fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags {
        flags
    }
}

xilem_core::generate_view_trait! {View, DomNode, Cx, ChangeFlags; (ViewMarker)}
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, DomNode, Cx, ChangeFlags, Pod;}
xilem_core::generate_anyview_trait! {AnyView, View, ViewMarker, Cx, ChangeFlags, AnyNode, BoxedView;}
xilem_core::generate_memoize_view! {Memoize, MemoizeState, View, ViewMarker, Cx, ChangeFlags, static_view, memoize;}
xilem_core::generate_adapt_view! {View, Cx, ChangeFlags;}
xilem_core::generate_adapt_state_view! {View, Cx, ChangeFlags;}

// strings -> text nodes

macro_rules! impl_string_view {
    ($ty:ty) => {
        impl ViewMarker for $ty {}
        impl<T, A> View<T, A> for $ty {
            type State = ();
            type Element = web_sys::Text;

            fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
                (Id::next(), (), new_text(self))
            }

            fn rebuild(
                &self,
                _cx: &mut Cx,
                prev: &Self,
                _id: &mut Id,
                _state: &mut Self::State,
                element: &mut Self::Element,
            ) -> ChangeFlags {
                if prev != self {
                    element.set_data(self);
                    ChangeFlags::OTHER_CHANGE
                } else {
                    ChangeFlags::empty()
                }
            }

            fn message(
                &self,
                _id_path: &[Id],
                _state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                _app_state: &mut T,
            ) -> MessageResult<A> {
                MessageResult::Stale(message)
            }
        }
    };
}

impl_string_view!(String);
impl_string_view!(&'static str);
impl_string_view!(Cow<'static, str>);

// Specialization would probably avoid manual implementation,
// but it's probably a good idea to have more control than via a blanket impl
macro_rules! impl_to_string_view {
    ($ty:ty) => {
        impl ViewMarker for $ty {}
        impl<T, A> View<T, A> for $ty {
            type State = ();
            type Element = web_sys::Text;

            fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
                (Id::next(), (), new_text(&self.to_string()))
            }

            fn rebuild(
                &self,
                _cx: &mut Cx,
                prev: &Self,
                _id: &mut Id,
                _state: &mut Self::State,
                element: &mut Self::Element,
            ) -> ChangeFlags {
                if prev != self {
                    element.set_data(&self.to_string());
                    ChangeFlags::OTHER_CHANGE
                } else {
                    ChangeFlags::empty()
                }
            }

            fn message(
                &self,
                _id_path: &[Id],
                _state: &mut Self::State,
                message: Box<dyn std::any::Any>,
                _app_state: &mut T,
            ) -> MessageResult<A> {
                MessageResult::Stale(message)
            }
        }
    };
}

// Allow numbers to be used directly as a view
impl_to_string_view!(f32);
impl_to_string_view!(f64);
impl_to_string_view!(i8);
impl_to_string_view!(u8);
impl_to_string_view!(i16);
impl_to_string_view!(u16);
impl_to_string_view!(i32);
impl_to_string_view!(u32);
impl_to_string_view!(i64);
impl_to_string_view!(u64);
impl_to_string_view!(u128);
impl_to_string_view!(isize);
impl_to_string_view!(usize);

fn new_text(text: &str) -> web_sys::Text {
    web_sys::Text::new_with_data(text).unwrap()
}
