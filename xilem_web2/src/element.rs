use wasm_bindgen::UnwrapThrowExt;

use crate::{attribute::Attributes, class::Classes, document, DynNode, Pod};

pub struct ElementProps {
    pub(crate) attributes: Attributes,
    pub(crate) classes: Classes,
    pub children: Vec<Pod<DynNode>>,
}

impl ElementProps {
    // All of this is slightly more complicated than it should be,
    // because we want to minimize DOM traffic as much as possible (that's basically the bottleneck)
    pub fn update_element(&mut self, element: &web_sys::Element) {
        self.attributes.apply_attribute_changes(element);
        self.classes.apply_class_changes(element);
    }
}

impl Pod<web_sys::Element> {
    pub fn new_element(children: Vec<Pod<DynNode>>, ns: &str, elem_name: &str) -> Self {
        let element = document()
            .create_element_ns(Some(ns), elem_name)
            .unwrap_throw();

        for child in children.iter() {
            let _ = element.append_child(child.node.as_ref());
        }

        Self {
            node: element,
            props: ElementProps {
                attributes: Attributes::default(),
                classes: Classes::default(),
                children,
            },
        }
    }
}
