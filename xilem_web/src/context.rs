// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::vecmap::VecMap;
#[cfg(feature = "hydration")]
use std::any::Any;
use std::any::TypeId;
use std::rc::Rc;

use crate::{
    app::{AppMessage, AppRunner},
    core::{ViewId, ViewPathTracker},
    Message,
};

/// A thunk to send messages to the views, it's being used for example in event callbacks
pub struct MessageThunk {
    id_path: Rc<[ViewId]>,
    app_ref: Box<dyn AppRunner>,
}

impl MessageThunk {
    /// Sends a message to the [`View`](`crate::core::View`) this thunk was being created in.
    /// One needs to be cautious with this being called synchronously, as this can produce a panic ("already mutably borrowed")
    ///
    /// # Panics
    ///
    /// When this is called synchronously (i.e. not via an event callback or by queuing it in the event loop with e.g. [`spawn_local`](`wasm_bindgen_futures::spawn_local`).
    pub fn push_message(&self, message_body: impl Message) {
        let message = AppMessage {
            id_path: Rc::clone(&self.id_path),
            body: Box::new(message_body),
        };
        self.app_ref.handle_message(message);
    }
}

/// The [`View`](`crate::core::View`) `Context` which is used for all [`DomView`](`crate::DomView`)s
pub struct ViewCtx {
    id_path: Vec<ViewId>,
    app_ref: Option<Box<dyn AppRunner>>,
    pub(crate) fragment: Rc<web_sys::DocumentFragment>,
    #[cfg(feature = "hydration")]
    hydration_node_stack: Vec<web_sys::Node>,
    #[cfg(feature = "hydration")]
    is_hydrating: bool,
    #[cfg(feature = "hydration")]
    pub(crate) templates: VecMap<TypeId, (web_sys::Node, Rc<dyn Any>)>,
    modifier_size_hints: VecMap<TypeId, usize>,
}

impl Default for ViewCtx {
    fn default() -> Self {
        ViewCtx {
            id_path: Vec::default(),
            app_ref: None,
            fragment: Rc::new(crate::document().create_document_fragment()),
            #[cfg(feature = "hydration")]
            templates: Default::default(),
            #[cfg(feature = "hydration")]
            hydration_node_stack: Default::default(),
            #[cfg(feature = "hydration")]
            is_hydrating: false,
            modifier_size_hints: Default::default(),
        }
    }
}

impl ViewCtx {
    /// Create a thunk to delay a message to the [`View`](`crate::core::View`) this was called in.
    pub fn message_thunk(&self) -> MessageThunk {
        MessageThunk {
            id_path: self.id_path.iter().copied().collect(),
            app_ref: self.app_ref.as_ref().unwrap().clone_box(),
        }
    }
    pub(crate) fn set_runner(&mut self, runner: impl AppRunner + 'static) {
        self.app_ref = Some(Box::new(runner));
    }

    #[cfg(feature = "hydration")]
    pub(crate) fn push_hydration_node(&mut self, node: web_sys::Node) {
        self.hydration_node_stack.push(node);
    }

    #[cfg(feature = "hydration")]
    pub(crate) fn enable_hydration(&mut self) {
        self.is_hydrating = true;
    }

    #[cfg(feature = "hydration")]
    pub(crate) fn disable_hydration(&mut self) {
        self.is_hydrating = false;
    }

    #[cfg(feature = "hydration")]
    pub(crate) fn is_hydrating(&self) -> bool {
        self.is_hydrating
    }

    #[cfg(feature = "hydration")]
    pub(crate) fn enter_hydrating_children(&mut self) {
        if let Some(node) = self.hydration_node_stack.last() {
            if let Some(child) = node.first_child() {
                self.hydration_node_stack.push(child);
            }
            // TODO panic else? Probably not, e.g. because of empty view sequences...
        }
    }

    #[cfg(feature = "hydration")]
    /// Returns the current node, and goes to the `next_sibling`, if it's `None`, it's popping the stack
    pub(crate) fn hydrate_node(&mut self) -> Option<web_sys::Node> {
        let node = self.hydration_node_stack.pop()?;
        if let Some(next_child) = node.next_sibling() {
            self.hydration_node_stack.push(next_child);
        }
        Some(node)
    }

    pub fn add_modifier_size_hint<T: 'static>(&mut self, request_size: usize) {
        let id = TypeId::of::<T>();
        match self.modifier_size_hints.get_mut(&id) {
            Some(hint) => *hint += request_size + 1, // + 1 because of the marker
            None => {
                self.modifier_size_hints.insert(id, request_size + 1);
            }
        };
    }

    pub fn modifier_size_hint<T: 'static>(&mut self) -> usize {
        match self.modifier_size_hints.get_mut(&TypeId::of::<T>()) {
            Some(hint) => std::mem::take(hint),
            None => 0,
        }
    }
}

impl ViewPathTracker for ViewCtx {
    fn push_id(&mut self, id: ViewId) {
        self.id_path.push(id);
    }

    fn pop_id(&mut self) {
        self.id_path.pop();
    }

    fn view_path(&mut self) -> &[ViewId] {
        &self.id_path
    }
}
