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

use std::{
    collections::HashSet,
    sync::{mpsc::SyncSender, Arc},
};

use futures_task::{ArcWake, Waker};

use xilem_core::{Id, IdPath};

use crate::widget::{AnyWidget, ChangeFlags, Pod, Widget};

xilem_core::generate_view_trait! {View <>, Widget, Cx, ChangeFlags; : Send}
xilem_core::generate_viewsequence_trait! {ViewSequence, View <>, ViewMarker, Widget, Cx, ChangeFlags, Pod; : Send}
xilem_core::generate_anyview_trait! {AnyView, View <>, ViewMarker, Cx, ChangeFlags, AnyWidget, BoxedView; + Send}
xilem_core::generate_memoize_view! {Memoize, MemoizeState, View <>, ViewMarker, Cx, ChangeFlags, s, memoize; + Send}
xilem_core::generate_adapt_view! {View <>, Cx, ChangeFlags; + Send}
xilem_core::generate_adapt_state_view! {View <>, Cx, ChangeFlags; + Send}

#[derive(Clone)]
pub struct Cx {
    id_path: IdPath,
    req_chan: SyncSender<IdPath>,
    pub(crate) pending_async: HashSet<Id>,
}

struct MyWaker {
    id_path: IdPath,
    req_chan: SyncSender<IdPath>,
}

impl ArcWake for MyWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        //println!("path = {:?}", arc_self.id_path);
        let _ = arc_self.req_chan.send(arc_self.id_path.clone());
    }
}

impl Cx {
    pub(crate) fn new(req_chan: &SyncSender<IdPath>) -> Self {
        Cx {
            id_path: Vec::new(),
            req_chan: req_chan.clone(),
            pending_async: HashSet::new(),
        }
    }

    pub fn push(&mut self, id: Id) {
        self.id_path.push(id);
    }

    pub fn pop(&mut self) {
        self.id_path.pop();
    }

    pub fn is_empty(&self) -> bool {
        self.id_path.is_empty()
    }

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

    pub fn waker(&self) -> Waker {
        futures_task::waker(Arc::new(MyWaker {
            id_path: self.id_path.clone(),
            req_chan: self.req_chan.clone(),
        }))
    }

    /// Add an id for a pending async future.
    ///
    /// Rendering may be delayed when there are pending async futures, to avoid
    /// flashing, and continues when all futures complete, or a timeout, whichever
    /// is first.
    pub fn add_pending_async(&mut self, id: Id) {
        self.pending_async.insert(id);
    }
}
