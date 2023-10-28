// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, rc::Rc};

use crate::{
    app::{AppMessage, AppRunner},
    core::{ViewId, ViewPathTracker},
    Message,
};

pub struct MessageThunk {
    id_path: Rc<[ViewId]>,
    app_ref: Box<dyn AppRunner>,
}

impl MessageThunk {
    pub fn push_message(&self, message_body: impl Message) {
        let message = AppMessage {
            id_path: Rc::clone(&self.id_path),
            body: Box::new(message_body),
        };
        self.app_ref.handle_message(message);
    }
}

/// The [`View`](`crate::core::View`) `Context` which is used for all [`DomView`](`crate::DomView`)s
#[derive(Default)]
pub struct ViewCtx {
    id_path: Vec<ViewId>,
    pub(crate) after_update: BTreeMap<ViewId, (bool, Vec<ViewId>)>,
    app_ref: Option<Box<dyn AppRunner>>,
}

impl ViewCtx {
    pub fn message_thunk(&self) -> MessageThunk {
        MessageThunk {
            id_path: self.id_path.iter().copied().collect(),
            app_ref: self.app_ref.as_ref().unwrap().clone_box(),
        }
    }
    pub(crate) fn set_runner(&mut self, runner: impl AppRunner + 'static) {
        self.app_ref = Some(Box::new(runner));
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
