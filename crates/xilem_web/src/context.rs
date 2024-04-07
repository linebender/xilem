use std::any::Any;

use crate::{app::AppRunner, view::DomNode, Message, Pod};
use bitflags::bitflags;
use web_sys::Document;
use xilem_core::{Id, IdPath};

// Note: xilem has derive Clone here. Not sure.
pub struct Cx {
    id_path: IdPath,
    document: Document,
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
