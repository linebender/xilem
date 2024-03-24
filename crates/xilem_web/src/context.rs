use std::any::Any;

use bitflags::bitflags;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::Document;

use xilem_core::{Id, IdPath};

use crate::{
    app::AppRunner,
    diff::{diff_kv_iterables, Diff},
    vecmap::VecMap,
    view::DomNode,
    AttributeValue, Message, Pod,
};

type CowStr = std::borrow::Cow<'static, str>;

fn set_attribute(element: &web_sys::Element, name: &str, value: &str) {
    // we have to special-case `value` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "value" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_value(value);
    } else if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(true);
    } else {
        element.set_attribute(name, value).unwrap_throw();
    }
}

fn remove_attribute(element: &web_sys::Element, name: &str) {
    // we have to special-case `checked` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(false);
    } else {
        element.remove_attribute(name).unwrap_throw();
    }
}

fn set_class(element: &web_sys::Element, class_name: &str) {
    #[cfg(debug_assertions)]
    if class_name.is_empty() {
        panic!("class names cannot be the empty string");
    }
    #[cfg(debug_assertions)]
    if class_name.contains(' ') {
        panic!("class names cannot contain the ascii space character");
    }
    element.class_list().add_1(class_name).unwrap_throw()
}

fn remove_class(element: &web_sys::Element, class_name: &str) {
    #[cfg(debug_assertions)]
    if class_name.is_empty() {
        panic!("class names cannot be the empty string");
    }
    #[cfg(debug_assertions)]
    if class_name.contains(' ') {
        panic!("class names cannot contain the ascii space character");
    }
    element.class_list().remove_1(class_name).unwrap_throw()
}

fn set_style(element: &web_sys::Element, name: &str, value: &str) {
    // styles will be ignored for non-html elements (e.g. SVG)
    if let Some(el) = element.dyn_ref::<web_sys::HtmlElement>() {
        el.style().set_property(name, value).unwrap_throw()
    }
}

fn remove_style(element: &web_sys::Element, name: &str) {
    // styles will be ignored for non-html elements (e.g. SVG)
    if let Some(el) = element.dyn_ref::<web_sys::HtmlElement>() {
        el.style().remove_property(name).unwrap_throw();
    }
}

// Note: xilem has derive Clone here. Not sure.
pub struct Cx {
    id_path: IdPath,
    document: Document,
    // TODO There's likely a cleaner more robust way to propagate the attributes to an element
    pub(crate) current_element_attributes: VecMap<CowStr, AttributeValue>,
    pub(crate) current_element_classes: VecMap<CowStr, ()>,
    pub(crate) current_element_styles: VecMap<CowStr, CowStr>,
    app_ref: Option<Box<dyn AppRunner>>,
}

pub struct MessageThunk {
    id_path: IdPath,
    app_ref: Box<dyn AppRunner>,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ChangeFlags: u32 {
        const STRUCTURE = 1;
        const OTHER_CHANGE = 2;
    }
}

impl Cx {
    pub fn new() -> Self {
        Cx {
            id_path: Vec::new(),
            document: crate::document(),
            app_ref: None,
            current_element_attributes: Default::default(),
            current_element_classes: Default::default(),
            current_element_styles: Default::default(),
        }
    }

    pub fn push(&mut self, id: Id) {
        self.id_path.push(id);
    }

    pub fn pop(&mut self) {
        self.id_path.pop();
    }

    #[allow(unused)]
    pub fn id_path(&self) -> &IdPath {
        &self.id_path
    }

    /// Run some logic with an id added to the id path.
    ///
    /// This is an ergonomic helper that ensures proper nesting of the id path.
    pub fn with_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, id: Id, f: F) -> T {
        self.push(id);
        let result = f(self);
        self.pop();
        result
    }

    /// Allocate a new id and run logic with the new id added to the id path.
    ///
    /// Also an ergonomic helper.
    pub fn with_new_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, f: F) -> (Id, T) {
        let id = Id::next();
        self.push(id);
        let result = f(self);
        self.pop();
        (id, result)
    }

    /// Run some logic within a new Pod context and return the newly created Pod,
    ///
    /// This logic is usually `View::build` to wrap the returned element into a Pod.
    pub fn with_new_pod<S, E, F>(&mut self, f: F) -> (Id, S, Pod)
    where
        E: DomNode,
        F: FnOnce(&mut Cx) -> (Id, S, E),
    {
        let (id, state, element) = f(self);
        (id, state, Pod::new(element))
    }

    /// Run some logic within the context of a given Pod,
    ///
    /// This logic is usually `View::rebuild`
    ///
    /// # Panics
    ///
    /// When the element type `E` is not the same type as the inner `DomNode` of the `Pod`
    pub fn with_pod<T, E, F>(&mut self, pod: &mut Pod, f: F) -> T
    where
        E: DomNode,
        F: FnOnce(&mut E, &mut Cx) -> T,
    {
        let element = pod
            .downcast_mut()
            .expect("Element type has changed, this should never happen!");
        f(element, self)
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub(crate) fn build_element(
        &mut self,
        ns: &str,
        name: &str,
    ) -> (
        web_sys::Element,
        VecMap<CowStr, AttributeValue>,
        VecMap<CowStr, ()>,
        VecMap<CowStr, CowStr>,
    ) {
        let el = self
            .document
            .create_element_ns(Some(ns), name)
            .expect("could not create element");
        let attributes = self.apply_attributes(&el);
        let classes = self.apply_classes(&el);
        let styles = self.apply_styles(&el);
        (el, attributes, classes, styles)
    }

    pub(crate) fn rebuild_element(
        &mut self,
        element: &web_sys::Element,
        attributes: &mut VecMap<CowStr, AttributeValue>,
        classes: &mut VecMap<CowStr, ()>,
        styles: &mut VecMap<CowStr, CowStr>,
    ) -> ChangeFlags {
        self.apply_attribute_changes(element, attributes)
            | self.apply_class_changes(element, classes)
            | self.apply_style_changes(element, styles)
    }

    // TODO Not sure how multiple attribute definitions with the same name should be handled (e.g. `e.attr("class", "a").attr("class", "b")`)
    // Currently the outer most (in the example above "b") defines the attribute (when it isn't `None`, in that case the inner attr defines the value)
    pub(crate) fn add_attr_to_element(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        // Special-case class so it works with the `class` method
        if name == "class" {
            if let Some(value) = value {
                let value = value.serialize();
                for class_name in value.split_ascii_whitespace() {
                    if !class_name.is_empty()
                        && !self.current_element_classes.contains_key(class_name)
                    {
                        self.current_element_classes
                            .insert(class_name.to_string().into(), ());
                    }
                }
            }
            return;
        }

        // parse styles
        if name == "style" {
            if let Some(value) = value {
                let value = value.serialize();
                for pair in value.split(';') {
                    let mut iter = pair.splitn(2, ':');
                    let Some(name) = iter.next() else {
                        continue;
                    };
                    let Some(value) = iter.next() else {
                        continue;
                    };
                    if name.is_empty() || value.is_empty() {
                        continue;
                    }
                    if !self.current_element_styles.contains_key(name) {
                        self.current_element_styles
                            .insert(name.to_string().into(), value.to_string().into());
                    }
                }
            }
            return;
        }

        if let Some(value) = value {
            // could be slightly optimized via something like this: `new_attrs.entry(name).or_insert_with(|| value)`
            if !self.current_element_attributes.contains_key(name) {
                self.current_element_attributes
                    .insert(name.clone(), value.clone());
            }
        }
    }

    pub(crate) fn add_class_to_element(&mut self, class_name: &CowStr) {
        // Don't strictly need this check but I assume its better for perf (might not be though)
        if !self.current_element_classes.contains_key(class_name) {
            self.current_element_classes.insert(class_name.clone(), ());
        }
    }

    pub(crate) fn add_style_to_element(&mut self, name: &CowStr, value: &CowStr) {
        if !self.current_element_styles.contains_key(name) {
            self.current_element_styles
                .insert(name.clone(), value.clone());
        }
    }

    pub(crate) fn apply_attributes(
        &mut self,
        element: &web_sys::Element,
    ) -> VecMap<CowStr, AttributeValue> {
        let mut attributes = VecMap::default();
        std::mem::swap(&mut attributes, &mut self.current_element_attributes);
        for (name, value) in attributes.iter() {
            set_attribute(element, name, &value.serialize());
        }
        attributes
    }

    pub(crate) fn apply_attribute_changes(
        &mut self,
        element: &web_sys::Element,
        attributes: &mut VecMap<CowStr, AttributeValue>,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::empty();
        // update attributes
        for itm in diff_kv_iterables(&*attributes, &self.current_element_attributes) {
            match itm {
                Diff::Add(name, value) | Diff::Change(name, value) => {
                    set_attribute(element, name, &value.serialize());
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
                Diff::Remove(name) => {
                    remove_attribute(element, name);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
            }
        }
        std::mem::swap(attributes, &mut self.current_element_attributes);
        self.current_element_attributes.clear();
        changed
    }

    pub(crate) fn apply_classes(&mut self, element: &web_sys::Element) -> VecMap<CowStr, ()> {
        let mut classes = VecMap::default();
        std::mem::swap(&mut classes, &mut self.current_element_classes);
        for (class_name, ()) in classes.iter() {
            set_class(element, class_name);
        }
        classes
    }

    pub(crate) fn apply_class_changes(
        &mut self,
        element: &web_sys::Element,
        classes: &mut VecMap<CowStr, ()>,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::empty();
        // update attributes
        for itm in diff_kv_iterables(&*classes, &self.current_element_classes) {
            match itm {
                Diff::Add(class_name, ()) | Diff::Change(class_name, ()) => {
                    set_class(element, class_name);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
                Diff::Remove(class_name) => {
                    remove_class(element, class_name);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
            }
        }
        std::mem::swap(classes, &mut self.current_element_classes);
        self.current_element_classes.clear();
        changed
    }

    pub(crate) fn apply_styles(&mut self, element: &web_sys::Element) -> VecMap<CowStr, CowStr> {
        let mut styles = VecMap::default();
        std::mem::swap(&mut styles, &mut self.current_element_styles);
        for (name, value) in styles.iter() {
            set_style(element, name, value);
        }
        styles
    }

    pub(crate) fn apply_style_changes(
        &mut self,
        element: &web_sys::Element,
        styles: &mut VecMap<CowStr, CowStr>,
    ) -> ChangeFlags {
        let mut changed = ChangeFlags::empty();
        // update attributes
        for itm in diff_kv_iterables(&*styles, &self.current_element_styles) {
            match itm {
                Diff::Add(name, value) | Diff::Change(name, value) => {
                    set_style(element, name, value);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
                Diff::Remove(name) => {
                    remove_style(element, name);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
            }
        }
        std::mem::swap(styles, &mut self.current_element_styles);
        self.current_element_styles.clear();
        changed
    }

    pub fn message_thunk(&self) -> MessageThunk {
        MessageThunk {
            id_path: self.id_path.clone(),
            app_ref: self.app_ref.as_ref().unwrap().clone_box(),
        }
    }
    pub(crate) fn set_runner(&mut self, runner: impl AppRunner + 'static) {
        self.app_ref = Some(Box::new(runner));
    }
}

impl Default for Cx {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageThunk {
    pub fn push_message(&self, message_body: impl Any + 'static) {
        let message = Message {
            id_path: self.id_path.clone(),
            body: Box::new(message_body),
        };
        self.app_ref.handle_message(message);
    }
}

impl ChangeFlags {
    pub fn tree_structure() -> Self {
        Self::STRUCTURE
    }
}
