// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Integration with xilem_core. This instantiates the View and related
//! traits for DOM node generation.

use std::{any::Any, borrow::Cow, ops::Deref};

use wasm_bindgen::throw_str;
use xilem_core::{Id, MessageResult};

use crate::{context::Cx, ChangeFlags};

mod sealed {
    pub trait Sealed {}
}

// A possible refinement of xilem_core is to allow a single concrete type
// for a view element, rather than an associated type with a bound.
/// This trait is implemented for types that implement `AsRef<web_sys::Node>`.
/// It is an implementation detail.
pub trait DomNode: sealed::Sealed {
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

/// This trait is implemented for types that implement `AsRef<web_sys::Element>`.
/// It is an implementation detail.
pub trait DomElement: DomNode {
    fn as_element_ref(&self) -> &web_sys::Element;
}

impl<N: DomNode + AsRef<web_sys::Element>> DomElement for N {
    fn as_element_ref(&self) -> &web_sys::Element {
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

xilem_core::generate_view_trait! {View, DomNode, Cx, ChangeFlags;}
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, DomNode, Cx, ChangeFlags, Pod;}
xilem_core::generate_anyview_trait! {AnyView, View, ViewMarker, Cx, ChangeFlags, AnyNode, BoxedView;}
xilem_core::generate_memoize_view! {Memoize, MemoizeState, View, ViewMarker, Cx, ChangeFlags, s, memoize}
xilem_core::impl_adapt_view! {View, Cx, ChangeFlags}

/// This view container can switch between two views.
///
/// It is a statically-typed alternative to the type-erased `AnyView`.
pub enum Either<T1, T2> {
    Left(T1),
    Right(T2),
}

impl<E1, E2> AsRef<web_sys::Node> for Either<E1, E2>
where
    E1: AsRef<web_sys::Node>,
    E2: AsRef<web_sys::Node>,
{
    fn as_ref(&self) -> &web_sys::Node {
        match self {
            Either::Left(view) => view.as_ref(),
            Either::Right(view) => view.as_ref(),
        }
    }
}

impl<T, A, V1, V2> View<T, A> for Either<V1, V2>
where
    V1: View<T, A>,
    V2: View<T, A>,
    V1::Element: AsRef<web_sys::Node> + 'static,
    V2::Element: AsRef<web_sys::Node> + 'static,
{
    type State = Either<V1::State, V2::State>;
    type Element = Either<V1::Element, V2::Element>;

    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        match self {
            Either::Left(view) => {
                let (id, state, el) = view.build(cx);
                (id, Either::Left(state), Either::Left(el))
            }
            Either::Right(view) => {
                let (id, state, el) = view.build(cx);
                (id, Either::Right(state), Either::Right(el))
            }
        }
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut change_flags = ChangeFlags::empty();
        match (prev, self) {
            (Either::Left(_), Either::Right(view)) => {
                let (new_id, new_state, new_element) = view.build(cx);
                *id = new_id;
                *state = Either::Right(new_state);
                *element = Either::Right(new_element);
                change_flags |= ChangeFlags::STRUCTURE;
            }
            (Either::Right(_), Either::Left(view)) => {
                let (new_id, new_state, new_element) = view.build(cx);
                *id = new_id;
                *state = Either::Left(new_state);
                *element = Either::Left(new_element);
                change_flags |= ChangeFlags::STRUCTURE;
            }
            (Either::Left(prev_view), Either::Left(view)) => {
                let (Either::Left(state), Either::Left(element)) = (state, element) else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                // Cannot do mutable casting, so take ownership of state.
                change_flags |= view.rebuild(cx, prev_view, id, state, element);
            }
            (Either::Right(prev_view), Either::Right(view)) => {
                let (Either::Right(state), Either::Right(element)) = (state, element) else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                // Cannot do mutable casting, so take ownership of state.
                change_flags |= view.rebuild(cx, prev_view, id, state, element);
            }
        }
        change_flags
    }

    fn message(
        &self,
        id_path: &[xilem_core::Id],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match self {
            Either::Left(view) => {
                let Either::Left(state) = state else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                view.message(id_path, state, message, app_state)
            }
            Either::Right(view) => {
                let Either::Right(state) = state else {
                    throw_str("invalid state/view in Either (unreachable)");
                };
                view.message(id_path, state, message, app_state)
            }
        }
    }
}

// strings -> text nodes

impl ViewMarker for &'static str {}
impl<T, A> View<T, A> for &'static str {
    type State = ();
    type Element = web_sys::Text;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = new_text(self);
        let id = Id::next();
        (id, (), el)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::empty();
        if prev != self {
            element.set_data(self);
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Nop
    }
}

impl ViewMarker for String {}
impl<T, A> View<T, A> for String {
    type State = ();
    type Element = web_sys::Text;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = new_text(self);
        let id = Id::next();
        (id, (), el)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::empty();
        if prev != self {
            element.set_data(self);
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Nop
    }
}

impl ViewMarker for Cow<'static, str> {}
impl<T, A> View<T, A> for Cow<'static, str> {
    type State = ();
    type Element = web_sys::Text;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = new_text(self);
        let id = Id::next();
        (id, (), el)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) -> ChangeFlags {
        let mut is_changed = ChangeFlags::empty();
        if prev != self {
            element.set_data(self);
            is_changed |= ChangeFlags::OTHER_CHANGE;
        }
        is_changed
    }

    fn message(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _message: Box<dyn std::any::Any>,
        _app_state: &mut T,
    ) -> MessageResult<A> {
        MessageResult::Nop
    }
}

fn new_text(text: &str) -> web_sys::Text {
    web_sys::Text::new_with_data(text).unwrap()
}
