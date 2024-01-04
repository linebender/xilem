use std::any::Any;

use bitflags::bitflags;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::Document;

use xilem_core::{Id, IdPath};

use crate::{
    app::AppRunner,
    diff::{diff_kv_iterables, Diff},
    vecmap::VecMap,
    AttributeValue, Message,
};

type CowStr = std::borrow::Cow<'static, str>;

fn set_attribute(element: &web_sys::Element, name: &str, value: &str) {
    // we have to special-case `value` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "value" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_value(value)
    } else if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(true)
    } else {
        element.set_attribute(name, value).unwrap_throw();
    }
}

fn remove_attribute(element: &web_sys::Element, name: &str) {
    // we have to special-case `checked` because setting the value using `set_attribute`
    // doesn't work after the value has been changed.
    if name == "checked" {
        let element: &web_sys::HtmlInputElement = element.dyn_ref().unwrap_throw();
        element.set_checked(false)
    } else {
        element.remove_attribute(name).unwrap_throw();
    }
}

// Note: xilem has derive Clone here. Not sure.
pub struct Cx {
    id_path: IdPath,
    document: Document,
    // TODO There's likely a cleaner more robust way to propagate the attributes to an element
    pub(crate) current_element_attributes: VecMap<CowStr, AttributeValue>,
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

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub(crate) fn build_element(
        &mut self,
        ns: &str,
        name: &str,
    ) -> (web_sys::Element, VecMap<CowStr, AttributeValue>) {
        let el = self
            .document
            .create_element_ns(Some(ns), name)
            .expect("could not create element");
        let attributes = self.apply_attributes(&el);
        (el, attributes)
    }

    pub(crate) fn rebuild_element(
        &mut self,
        element: &web_sys::Element,
        attributes: &mut VecMap<CowStr, AttributeValue>,
    ) -> ChangeFlags {
        self.apply_attribute_changes(element, attributes)
    }

    // TODO Not sure how multiple attribute definitions with the same name should be handled (e.g. `e.attr("class", "a").attr("class", "b")`)
    // Currently the outer most (in the example above "b") defines the attribute (when it isn't `None`, in that case the inner attr defines the value)
    pub(crate) fn add_attr_to_element(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        if let Some(value) = value {
            // could be slightly optimized via something like this: `new_attrs.entry(name).or_insert_with(|| value)`
            if !self.current_element_attributes.contains_key(name) {
                self.current_element_attributes
                    .insert(name.clone(), value.clone());
            }
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
