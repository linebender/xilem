use wasm_bindgen::UnwrapThrowExt;

use crate::{attribute::Attributes, class::ClassAttributes, document, DynNode, Pod};

// #[derive(Debug)]
pub struct ElementAttributes {
    pub(crate) attributes: Attributes,
    pub(crate) class_attributes: ClassAttributes,
    pub children: Vec<Pod<DynNode>>,
}

pub trait AppliableAttributes<E> {
    fn apply_attributes(&mut self, element: &E);
}

impl<E: AsRef<web_sys::Element>> AppliableAttributes<E> for ElementAttributes {
    // type E = E;

    fn apply_attributes(&mut self, element: &E) {
        self.apply_attributes(element.as_ref());
    }
}

impl ElementAttributes {
    // All of this is slightly more complicated than it should be,
    // because we want to minimize DOM traffic as much as possible (that's basically the bottleneck)
    pub fn apply_attributes(&mut self, element: &web_sys::Element) {
        self.attributes.apply_attribute_changes(element);
        self.class_attributes.apply_class_changes(element);
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
            attrs: ElementAttributes {
                attributes: Attributes::default(),
                class_attributes: ClassAttributes::default(),
                children,
            },
        }
    }
}
