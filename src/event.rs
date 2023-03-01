// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::any::Any;

use crate::id::IdPath;

pub struct Message {
    pub id_path: IdPath,
    pub body: Box<dyn Any + Send>,
}

/// A result wrapper type for event handlers.
pub enum MessageResult<A> {
    /// The event handler was invoked and returned an action.
    Action(A),
    /// The event handler received a change request that requests a rebuild.
    #[allow(unused)]
    RequestRebuild,
    /// The event handler discarded the event.
    #[allow(unused)]
    Nop,
    /// The event was addressed to an id path no longer in the tree.
    ///
    /// This is a normal outcome for async operation when the tree is changing
    /// dynamically, but otherwise indicates a logic error.
    Stale(Box<dyn Any>),
}

pub struct AsyncWake;

impl<A> MessageResult<A> {
    #[allow(unused)]
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> MessageResult<B> {
        match self {
            MessageResult::Action(a) => MessageResult::Action(f(a)),
            MessageResult::RequestRebuild => MessageResult::RequestRebuild,
            MessageResult::Stale(event) => MessageResult::Stale(event),
            MessageResult::Nop => MessageResult::Nop,
        }
    }

    pub fn or(self, f: impl FnOnce(Box<dyn Any>) -> Self) -> Self {
        match self {
            MessageResult::Stale(event) => f(event),
            _ => self,
        }
    }
}

impl Message {
    pub fn new(id_path: IdPath, event: impl Any + Send) -> Message {
        Message {
            id_path,
            body: Box::new(event),
        }
    }
}
