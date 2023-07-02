// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Integration with xilem_core. This instantiates the View and related
//! traits for DOM node generation.

use std::{any::Any, borrow::Cow, ops::Deref};

use xilem_core::{Id, MessageResult};

use crate::{context::Cx, ChangeFlags};

// A possible refinement of xilem_core is to allow a single concrete type
// for a view element, rather than an associated type with a bound.
pub trait DomNode {
    fn into_pod(self) -> Pod;
    fn as_node_ref(&self) -> &web_sys::Node;
}

impl<N: AsRef<web_sys::Node> + 'static> DomNode for N {
    fn into_pod(self) -> Pod {
        Pod(Box::new(self))
    }

    fn as_node_ref(&self) -> &web_sys::Node {
        self.as_ref()
    }
}

pub trait DomElement: DomNode {
    fn as_element_ref(&self) -> &web_sys::Element;
}

impl<N: DomNode + AsRef<web_sys::Element>> DomElement for N {
    fn as_element_ref(&self) -> &web_sys::Element {
        self.as_ref()
    }
}

pub trait AnyNode {
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

    fn downcast_mut<'a, T: 'static>(&'a mut self) -> Option<&'a mut T> {
        self.0.as_any_mut().downcast_mut()
    }

    fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags {
        flags
    }
}

xilem_core::generate_view_trait! {View, DomNode, Cx, ChangeFlags;}
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, DomNode, Cx, ChangeFlags, Pod;}
xilem_core::generate_anyview_trait! {View, Cx, ChangeFlags, AnyNode}

impl ViewMarker for &'static str {}
impl<T, A> View<T, A> for &'static str {
    type State = ();
    type Element = web_sys::Text;

    fn build(&self, _cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let el = new_text(self);
        let id = Id::next();
        (id, (), el.into())
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
        (id, (), el.into())
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
        (id, (), el.into())
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
