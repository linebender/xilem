// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

//! Integration with xilem_core. This instantiates the View and related
//! traits for DOM node generation.

use std::ops::Deref;

use crate::{context::Cx, ChangeFlags};

// A possible refinement of xilem_core is to allow a single concrete type
// for a view element, rather than an associated type with a bound.
pub trait DomElement {
    fn into_pod(self) -> Pod;
    fn as_element_ref(&self) -> &web_sys::Element;
}

pub trait AnyElement {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    fn as_element_ref(&self) -> &web_sys::Element;
}

impl AnyElement for web_sys::Element {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_element_ref(&self) -> &web_sys::Element {
        self
    }
}

impl DomElement for web_sys::Element {
    fn into_pod(self) -> Pod {
        Pod(Box::new(self))
    }

    fn as_element_ref(&self) -> &web_sys::Element {
        self
    }
}

impl DomElement for Box<dyn AnyElement> {
    fn into_pod(self) -> Pod {
        Pod(self)
    }

    fn as_element_ref(&self) -> &web_sys::Element {
        self.deref().as_element_ref()
    }
}

/// A container that holds a DOM element.
///
/// This implementation may be overkill (it's possibly enough that everything is
/// just a `web_sys::Element`), but does allow element types that contain other
/// data, if needed.
pub struct Pod(pub Box<dyn AnyElement>);

impl Pod {
    fn new(node: impl DomElement) -> Self {
        node.into_pod()
    }

    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.0.as_any_mut().downcast_mut()
    }

    fn mark(&mut self, flags: ChangeFlags) -> ChangeFlags {
        flags
    }
}

xilem_core::generate_view_trait! {View, DomElement, Cx, ChangeFlags;}
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, DomElement, Cx, ChangeFlags, Pod;}
xilem_core::generate_anyview_trait! {AnyView, View, ViewMarker, Cx, ChangeFlags, AnyElement, BoxedView;}
xilem_core::generate_memoize_view! {Memoize, MemoizeState, View, ViewMarker, Cx, ChangeFlags, s, memoize}
