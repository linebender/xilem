// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{attribute::Attributes, class::Classes, document, style::Styles, AnyPod, Pod, ViewCtx};
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
    pub fn new(
        children: Vec<AnyPod>,
        attr_size_hint: usize,
        style_size_hint: usize,
        class_size_hint: usize,
        #[cfg(feature = "hydration")] in_hydration: bool,
    ) -> Self {
        let attributes = if attr_size_hint > 0 {
            Some(Box::new(Attributes::new(
                attr_size_hint,
                #[cfg(feature = "hydration")]
                in_hydration,
            )))
        } else {
            None
        };
        let styles = if style_size_hint > 0 {
            Some(Box::new(Styles::new(
                style_size_hint,
                #[cfg(feature = "hydration")]
                in_hydration,
            )))
        } else {
            None
        };
        let classes = if class_size_hint > 0 {
            Some(Box::new(Classes::new(
                class_size_hint,
                #[cfg(feature = "hydration")]
                in_hydration,
            )))
        } else {
            None
        };
        Self {
            attributes,
            classes,
            styles,
            children,
            #[cfg(feature = "hydration")]
            in_hydration,
        }
    }

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
        self.attributes.get_or_insert_with(|| {
            Box::new(Attributes::new(
                0,
                #[cfg(feature = "hydration")]
                self.in_hydration,
            ))
        })
    }

    pub fn styles(&mut self) -> &mut Styles {
        self.styles.get_or_insert_with(|| {
            Box::new(Styles::new(
                0,
                #[cfg(feature = "hydration")]
                self.in_hydration,
            ))
        })
    }

    pub fn classes(&mut self) -> &mut Classes {
        self.classes.get_or_insert_with(|| {
            Box::new(Classes::new(
                0,
                #[cfg(feature = "hydration")]
                self.in_hydration,
            ))
        })
    }
}

impl Pod<web_sys::Element> {
    pub fn new_element_with_ctx(
        children: Vec<AnyPod>,
        ns: &str,
        elem_name: &str,
        ctx: &mut ViewCtx,
    ) -> Self {
        let attr_size_hint = ctx.modifier_size_hint::<Attributes>();
        let class_size_hint = ctx.modifier_size_hint::<Classes>();
        let style_size_hint = ctx.modifier_size_hint::<Styles>();
        Self::new_element(
            children,
            ns,
            elem_name,
            attr_size_hint,
            style_size_hint,
            class_size_hint,
        )
    }

    /// Creates a new Pod with [`web_sys::Element`] as element and `ElementProps` as its [`DomView::Props`](`crate::DomView::Props`)
    pub fn new_element(
        children: Vec<AnyPod>,
        ns: &str,
        elem_name: &str,
        attr_size_hint: usize,
        style_size_hint: usize,
        class_size_hint: usize,
    ) -> Self {
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
            props: ElementProps::new(
                children,
                attr_size_hint,
                style_size_hint,
                class_size_hint,
                #[cfg(feature = "hydration")]
                false,
            ),
        }
    }

    #[cfg(feature = "hydration")]
    pub fn hydrate_element_with_ctx(
        children: Vec<AnyPod>,
        element: web_sys::Node,
        ctx: &mut ViewCtx,
    ) -> Self {
        let attr_size_hint = ctx.modifier_size_hint::<Attributes>();
        let class_size_hint = ctx.modifier_size_hint::<Classes>();
        let style_size_hint = ctx.modifier_size_hint::<Styles>();
        Self::hydrate_element(
            children,
            element,
            attr_size_hint,
            style_size_hint,
            class_size_hint,
        )
    }

    #[cfg(feature = "hydration")]
    pub fn hydrate_element(
        children: Vec<AnyPod>,
        element: web_sys::Node,
        attr_size_hint: usize,
        style_size_hint: usize,
        class_size_hint: usize,
    ) -> Self {
        Self {
            node: element.unchecked_into(),
            props: ElementProps::new(
                children,
                attr_size_hint,
                style_size_hint,
                class_size_hint,
                #[cfg(feature = "hydration")]
                true,
            ),
        }
    }
}
