// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{attribute::Attributes, class::Classes, document, style::Styles, AnyPod, Pod};
#[cfg(feature = "hydration")]
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;

// Lazy access to attributes etc. to avoid allocating unnecessary memory when it isn't needed
// Benchmarks have shown, that this can significantly increase performance and reduce memory usage...
/// This holds all the state for a DOM [`Element`](`crate::interfaces::Element`), it is used for [`DomView::Props`](`crate::DomView::Props`)
pub struct ElementProps {
    #[cfg(feature = "hydration")]
    pub(crate) in_hydration: bool,
    pub(crate) attributes: Option<Box<Attributes>>,
    pub(crate) classes: Option<Box<Classes>>,
    pub(crate) styles: Option<Box<Styles>>,
    pub(crate) children: Vec<AnyPod>,
}

impl ElementProps {
    // All of this is slightly more complicated than it should be,
    // because we want to minimize DOM traffic as much as possible (that's basically the bottleneck)
    pub fn update_element(&mut self, element: &web_sys::Element) {
        if let Some(attributes) = &mut self.attributes {
            attributes.apply_attribute_changes(element);
        }
        if let Some(classes) = &mut self.classes {
            classes.apply_class_changes(element);
        }
        if let Some(styles) = &mut self.styles {
            styles.apply_style_changes(element);
        }
    }

    pub fn attributes(&mut self) -> &mut Attributes {
        #[cfg(feature = "hydration")]
        let attributes = self
            .attributes
            .get_or_insert_with(|| Box::new(Attributes::new(self.in_hydration)));
        #[cfg(not(feature = "hydration"))]
        // still unstable, but this would even be more concise
        // self.attributes.get_or_insert_default()
        let attributes = self.attributes.get_or_insert_with(Default::default);
        attributes
    }

    pub fn styles(&mut self) -> &mut Styles {
        #[cfg(feature = "hydration")]
        let styles = self
            .styles
            .get_or_insert_with(|| Box::new(Styles::new(self.in_hydration)));
        #[cfg(not(feature = "hydration"))]
        let styles = self.styles.get_or_insert_with(Default::default);
        styles
    }

    pub fn classes(&mut self) -> &mut Classes {
        #[cfg(feature = "hydration")]
        let classes = self
            .classes
            .get_or_insert_with(|| Box::new(Classes::new(self.in_hydration)));
        #[cfg(not(feature = "hydration"))]
        let classes = self.classes.get_or_insert_with(Default::default);
        classes
    }
}

impl Pod<web_sys::Element> {
    /// Creates a new Pod with [`web_sys::Element`] as element and `ElementProps` as its [`DomView::Props`](`crate::DomView::Props`)
    pub fn new_element(children: Vec<AnyPod>, ns: &str, elem_name: &str) -> Self {
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
            props: ElementProps {
                #[cfg(feature = "hydration")]
                in_hydration: false,
                attributes: None,
                classes: None,
                styles: None,
                children,
            },
        }
    }

    #[cfg(feature = "hydration")]
    pub fn hydrate_element(children: Vec<AnyPod>, element: web_sys::Node) -> Self {
        Self {
            node: element.unchecked_into(),
            props: ElementProps {
                in_hydration: true,
                attributes: None,
                classes: None,
                styles: None,
                children,
            },
        }
    }
}
