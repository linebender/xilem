// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Integration with xilem_core. This instantiates the View and related
//! traits for DOM node generation.

use std::{any::Any, ops::Deref};

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

struct Void;

// Dummy implementation that should never be used.
impl DomNode for Void {
    fn into_pod(self) -> Pod {
        unreachable!()
    }

    fn as_node_ref(&self) -> &web_sys::Node {
        unreachable!()
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
