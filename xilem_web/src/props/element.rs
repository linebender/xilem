// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    document,
    modifiers::{Attributes, Children, Classes, Styles, With},
    AnyPod, Pod, ViewCtx,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;

// Lazy access to attributes etc. to avoid allocating unnecessary memory when it isn't needed
// Benchmarks have shown, that this can significantly increase performance and reduce memory usage...
/// This holds all the state for a DOM [`Element`](`crate::interfaces::Element`), it is used for [`DomNode::Props`](`crate::DomNode::Props`)
pub struct Element {
    pub(crate) in_hydration: bool,
    pub(crate) attributes: Option<Box<Attributes>>,
    pub(crate) classes: Option<Box<Classes>>,
    pub(crate) styles: Option<Box<Styles>>,
    pub(crate) children: Vec<AnyPod>,
}

impl Element {
    pub fn new(
        children: Vec<AnyPod>,
        attr_size_hint: usize,
        style_size_hint: usize,
        class_size_hint: usize,
        in_hydration: bool,
    ) -> Self {
        Self {
            attributes: (attr_size_hint > 0)
                .then(|| Box::new(Attributes::new(attr_size_hint, in_hydration))),
            classes: (class_size_hint > 0)
                .then(|| Box::new(Classes::new(class_size_hint, in_hydration))),
            styles: (style_size_hint > 0)
                .then(|| Box::new(Styles::new(style_size_hint, in_hydration))),
            children,
            in_hydration,
        }
    }

    // All of this is slightly more complicated than it should be,
    // because we want to minimize DOM traffic as much as possible (that's basically the bottleneck)
    pub fn update_element(&mut self, element: &web_sys::Element) {
        if let Some(attributes) = &mut self.attributes {
            attributes.apply_changes(element);
        }
        if let Some(classes) = &mut self.classes {
            classes.apply_changes(element);
        }
        if let Some(styles) = &mut self.styles {
            styles.apply_changes(element);
        }
    }

    /// Lazily returns the [`Attributes`] modifier of this element.
    pub fn attributes(&mut self) -> &mut Attributes {
        self.attributes
            .get_or_insert_with(|| Box::new(Attributes::new(0, self.in_hydration)))
    }

    /// Lazily returns the [`Styles`] modifier of this element.
    pub fn styles(&mut self) -> &mut Styles {
        self.styles
            .get_or_insert_with(|| Box::new(Styles::new(0, self.in_hydration)))
    }

    /// Lazily returns the [`Classes`] modifier of this element.
    pub fn classes(&mut self) -> &mut Classes {
        self.classes
            .get_or_insert_with(|| Box::new(Classes::new(0, self.in_hydration)))
    }
}

impl Pod<web_sys::Element> {
    /// Creates a new Pod with [`web_sys::Element`] as element and `ElementProps` as its [`DomNode::Props`](`crate::DomNode::Props`).
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

        Self {
            node: element,
            props: Element::new(
                children,
                attr_size_hint,
                style_size_hint,
                class_size_hint,
                false,
            ),
        }
    }

    /// Creates a new Pod that hydrates an existing node (within the `ViewCtx`) as [`web_sys::Element`] and [`ElementProps`] as its [`DomNode::Props`](`crate::DomNode::Props`).
    pub fn hydrate_element_with_ctx(children: Vec<AnyPod>, ctx: &mut ViewCtx) -> Self {
        let attr_size_hint = ctx.take_modifier_size_hint::<Attributes>();
        let class_size_hint = ctx.take_modifier_size_hint::<Classes>();
        let style_size_hint = ctx.take_modifier_size_hint::<Styles>();
        let element = ctx.hydrate_node().unwrap_throw();

        Self {
            node: element.unchecked_into(),
            props: Element::new(
                children,
                attr_size_hint,
                style_size_hint,
                class_size_hint,
                true,
            ),
        }
    }
}

impl With<Children> for Element {
    fn modifier(&mut self) -> &mut Children {
        &mut self.children
    }
}

impl With<Attributes> for Element {
    fn modifier(&mut self) -> &mut Attributes {
        self.attributes()
    }
}

impl With<Classes> for Element {
    fn modifier(&mut self) -> &mut Classes {
        self.classes()
    }
}

impl With<Styles> for Element {
    fn modifier(&mut self) -> &mut Styles {
        self.styles()
    }
}
