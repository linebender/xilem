// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::{Any, TypeId};
use std::rc::Rc;

use wasm_bindgen_futures::spawn_local;

use crate::app::{AppMessage, AppRunner};
use crate::core::{ViewId, ViewPathTracker};
use crate::vecmap::VecMap;
use crate::Message;

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
    /// When this is called synchronously (i.e. not via an event callback or by enqueuing it in the event loop with e.g. [`spawn_local`](`wasm_bindgen_futures::spawn_local`).
    /// Use [`MessageThunk::enqueue_message`] instead in this case.
    pub fn push_message(&self, message_body: impl Message) {
        let message = AppMessage {
            id_path: Rc::clone(&self.id_path),
            body: Box::new(message_body),
        };
        self.app_ref.handle_message(message);
    }

    /// Sends a message to the [`View`](`crate::core::View`) this thunk was being created in. This is similar as [`MessageThunk::push_message`] but enqueues the message as next microtask.
    pub fn enqueue_message(&self, message_body: impl Message) {
        let message = AppMessage {
            id_path: Rc::clone(&self.id_path),
            body: Box::new(message_body),
        };
        let app_ref = self.app_ref.clone_box();
        spawn_local(async move { app_ref.handle_message(message) });
    }
}

/// The [`View`](`crate::core::View`) `Context` which is used for all [`DomView`](`crate::DomView`)s
pub struct ViewCtx {
    id_path: Vec<ViewId>,
    app_ref: Option<Box<dyn AppRunner>>,
    pub(crate) fragment: Rc<web_sys::DocumentFragment>,
    hydration_node_stack: Vec<web_sys::Node>,
    is_hydrating: bool,
    pub(crate) templates: VecMap<TypeId, (web_sys::Node, Rc<dyn Any>)>,
    /// A stack containing modifier count size-hints for each element context, mostly to avoid unnecessary allocations.
    modifier_size_hints: Vec<VecMap<TypeId, usize>>,
    modifier_size_hint_stack_idx: usize,
}

impl Default for ViewCtx {
    fn default() -> Self {
        Self {
            id_path: Vec::default(),
            app_ref: None,
            fragment: Rc::new(crate::document().create_document_fragment()),
            templates: VecMap::default(),
            hydration_node_stack: Vec::default(),
            is_hydrating: false,
            // One element for the root `DomFragment`. will be extended with `Self::push_size_hints`
            modifier_size_hints: vec![VecMap::default()],
            modifier_size_hint_stack_idx: 0,
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

    /// Provides a way to access this context without having to deal with lifetime issues.
    ///
    /// One Motivation for this is to allow custom view contexts with access to the app runtime.
    ///
    /// # Examples
    /// ```ignore
    /// struct MyContext { parent: ViewCtx };
    /// ctx.as_owned(|ctx| {
    ///     let my_ctx = MyContext { parent: ctx };
    ///     self.child_view_with_different_ctx.build(&mut my_ctx);
    ///     (my_ctx.parent, ())
    /// });
    /// ```
    pub fn as_owned<R>(&mut self, f: impl FnOnce(Self) -> (Self, R)) -> R {
        let temporary_owned_ctx = Self {
            id_path: std::mem::take(&mut self.id_path),
            app_ref: self.app_ref.as_ref().map(|app| app.clone_box()),
            fragment: self.fragment.clone(),
            hydration_node_stack: std::mem::take(&mut self.hydration_node_stack),
            is_hydrating: self.is_hydrating,
            templates: std::mem::take(&mut self.templates),
            modifier_size_hints: std::mem::take(&mut self.modifier_size_hints),
            modifier_size_hint_stack_idx: self.modifier_size_hint_stack_idx,
        };
        let (ctx, retval) = f(temporary_owned_ctx);
        self.id_path = ctx.id_path;
        self.hydration_node_stack = ctx.hydration_node_stack;
        self.is_hydrating = ctx.is_hydrating;
        self.templates = ctx.templates;
        self.modifier_size_hints = ctx.modifier_size_hints;
        self.modifier_size_hint_stack_idx = ctx.modifier_size_hint_stack_idx;
        retval
    }

    /// Should be used when creating children of a DOM node, e.g. to handle hydration and size hints correctly.
    pub fn with_build_children<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.enter_hydrating_children();
        self.push_size_hints();
        let r = f(self);
        self.pop_size_hints();
        r
    }

    pub fn with_hydration_node<R>(
        &mut self,
        node: web_sys::Node,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.hydration_node_stack.push(node);
        let is_hydrating = self.is_hydrating;
        self.is_hydrating = true;
        let r = f(self);
        self.is_hydrating = is_hydrating;
        r
    }

    pub(crate) fn is_hydrating(&self) -> bool {
        self.is_hydrating
    }

    fn enter_hydrating_children(&mut self) {
        if let Some(node) = self.hydration_node_stack.last() {
            if let Some(child) = node.first_child() {
                self.hydration_node_stack.push(child);
            }
            // TODO panic else? Probably not, e.g. because of empty view sequences...
        }
    }

    /// Returns the current node, and goes to the `next_sibling`, if it's `None`, it's popping the stack
    pub(crate) fn hydrate_node(&mut self) -> Option<web_sys::Node> {
        let node = self.hydration_node_stack.pop()?;
        if let Some(next_child) = node.next_sibling() {
            self.hydration_node_stack.push(next_child);
        }
        Some(node)
    }

    fn current_size_hints_mut(&mut self) -> &mut VecMap<TypeId, usize> {
        &mut self.modifier_size_hints[self.modifier_size_hint_stack_idx]
    }

    fn add_modifier_size_hint<T: 'static>(&mut self, request_size: usize) {
        let id = TypeId::of::<T>();
        let hints = self.current_size_hints_mut();
        match hints.get_mut(&id) {
            Some(hint) => *hint += request_size,
            None => {
                hints.insert(id, request_size);
            }
        }
    }

    #[inline]
    pub fn take_modifier_size_hint<T: 'static>(&mut self) -> usize {
        self.current_size_hints_mut()
            .get_mut(&TypeId::of::<T>())
            .map(std::mem::take)
            .unwrap_or(0)
    }

    fn push_size_hints(&mut self) {
        if self.modifier_size_hint_stack_idx == self.modifier_size_hints.len() - 1 {
            self.modifier_size_hints.push(VecMap::default());
        }
        self.modifier_size_hint_stack_idx += 1;
    }

    fn pop_size_hints(&mut self) {
        debug_assert!(
            self.modifier_size_hints[self.modifier_size_hint_stack_idx]
                .iter()
                .map(|(_, size_hint)| *size_hint)
                .sum::<usize>()
                == 0
        );
        self.modifier_size_hint_stack_idx -= 1;
    }

    #[inline]
    pub fn with_size_hint<T: 'static, R>(
        &mut self,
        size: usize,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.add_modifier_size_hint::<T>(size);
        f(self)
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
