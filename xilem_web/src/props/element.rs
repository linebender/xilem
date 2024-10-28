// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    document,
    events::Events,
    modifiers::{Attributes, Children, Classes, Modifier, Styles},
    AnyPod, Pod, PodFlags, ViewCtx,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;

// Lazy access to attributes etc. to avoid allocating unnecessary memory when it isn't needed
// Benchmarks have shown, that this can significantly increase performance and reduce memory usage...
/// This holds all the state for a DOM [`Element`](`crate::interfaces::Element`), it is used for [`DomNode::Props`](`crate::DomNode::Props`)
pub struct Element {
    pub(crate) attributes: Option<Box<Attributes>>,
    pub(crate) classes: Option<Box<Classes>>,
    pub(crate) styles: Option<Box<Styles>>,
    pub(crate) children: Vec<AnyPod>,
    pub(crate) events: Events,
}

impl Element {
    pub fn new(
        children: Vec<AnyPod>,
        attr_size_hint: usize,
        style_size_hint: usize,
        class_size_hint: usize,
    ) -> Self {
        Self {
            attributes: (attr_size_hint > 0).then(|| Attributes::new(attr_size_hint).into()),
            classes: (class_size_hint > 0).then(|| Classes::new(class_size_hint).into()),
            styles: (style_size_hint > 0).then(|| Styles::new(style_size_hint).into()),
            children,
            events: Events,
        }
    }

    // All of this is slightly more complicated than it should be,
    // because we want to minimize DOM traffic as much as possible (that's basically the bottleneck)
    pub fn update_element(&mut self, element: &web_sys::Element, flags: &mut PodFlags) {
        if let Some(attributes) = &mut self.attributes {
            Attributes::apply_changes(Modifier::new(attributes, flags), element);
        }
        if let Some(classes) = &mut self.classes {
            Classes::apply_changes(Modifier::new(classes, flags), element);
        }
        if let Some(styles) = &mut self.styles {
            Styles::apply_changes(Modifier::new(styles, flags), element);
        }
    }
}

impl Pod<web_sys::Element> {
    /// Creates a new Pod with [`web_sys::Element`] as element and [`Element`] as its [`DomNode::Props`](`crate::DomNode::Props`).
    pub fn new_element_with_ctx(
        children: Vec<AnyPod>,
        ns: &str,
        elem_name: &str,
        ctx: &mut ViewCtx,
    ) -> Self {
        let attr_size_hint = ctx.take_modifier_size_hint::<Attributes>();
        let class_size_hint = ctx.take_modifier_size_hint::<Classes>();
        let style_size_hint = ctx.take_modifier_size_hint::<Styles>();
        let element = document()
            .create_element_ns(
                Some(wasm_bindgen::intern(ns)),
                wasm_bindgen::intern(elem_name),
            )
            .unwrap_throw();

        for child in children.iter() {
            let _ = element.append_child(child.node.as_ref());
        }

        let props = Element::new(children, attr_size_hint, style_size_hint, class_size_hint);
        Pod::new(element.unchecked_into(), props, PodFlags::new(false))
    }

    /// Creates a new Pod that hydrates an existing node (within the `ViewCtx`) as [`web_sys::Element`] and [`Element`] as its [`DomNode::Props`](`crate::DomNode::Props`).
    pub fn hydrate_element_with_ctx(children: Vec<AnyPod>, ctx: &mut ViewCtx) -> Self {
        let attr_size_hint = ctx.take_modifier_size_hint::<Attributes>();
        let class_size_hint = ctx.take_modifier_size_hint::<Classes>();
        let style_size_hint = ctx.take_modifier_size_hint::<Styles>();
        let element = ctx.hydrate_node().unwrap_throw();

        let props = Element::new(children, attr_size_hint, style_size_hint, class_size_hint);
        Pod::new(element.unchecked_into(), props, PodFlags::new(true))
    }
}

impl AsMut<Children> for Element {
    fn as_mut(&mut self) -> &mut Children {
        &mut self.children
    }
}

impl AsMut<Attributes> for Element {
    fn as_mut(&mut self) -> &mut Attributes {
        self.attributes
            .get_or_insert_with(|| Attributes::new(0).into())
    }
}

impl AsMut<Classes> for Element {
    fn as_mut(&mut self) -> &mut Classes {
        self.classes.get_or_insert_with(|| Classes::new(0).into())
    }
}

impl AsMut<Styles> for Element {
    fn as_mut(&mut self) -> &mut Styles {
        self.styles.get_or_insert_with(|| Styles::new(0).into())
    }
}

impl AsMut<Events> for Element {
    fn as_mut(&mut self) -> &mut Events {
        &mut self.events
    }
}

/// An alias trait to sum up all modifiers that a DOM `Element` can have. It's used to avoid a lot of boilerplate in public APIs.
pub trait WithElementProps:
    AsMut<Attributes> + AsMut<Children> + AsMut<Classes> + AsMut<Styles>
{
}
impl<T: AsMut<Attributes> + AsMut<Children> + AsMut<Classes> + AsMut<Styles>> WithElementProps
    for T
{
}
