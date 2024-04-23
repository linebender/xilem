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

use masonry::{
    widget::{StoreInWidgetMut, WidgetMut},
    EventCtx, Widget, WidgetId, WidgetPod,
};
use xilem_core::{Id, IdPath};

use crate::widget::{tree_structure::TreeStructure, ChangeFlags};

/// A view object representing a node in the UI.
///
/// This is a central trait for representing UI. An app will generate a tree of
/// these objects (the view tree) as the primary interface for expressing UI.
/// The view tree is transitory and is retained only long enough to dispatch
/// messages and then serve as a reference for diffing for the next view tree.
///
/// The framework will then run methods on these views to create the associated
/// state tree and element tree, as well as incremental updates and message
/// propagation.
///
/// The `View` trait is parameterized by `T`, which is known as the "app state",
/// and also a type for actions which are passed up the tree in message
/// propagation. During message handling, mutable access to the app state is
/// given to view nodes, which in turn can expose it to callbacks.
pub trait View<T, A = ()>: Send {
    /// Associated state for the view.
    type State: Send;
    /// The associated element for the view.
    type Element: Widget + StoreInWidgetMut;
    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element);

    /// Update the associated element.
    ///
    /// Returns an indication of what, if anything, has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut WidgetMut<'_, Self::Element>,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this view.
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> crate::MessageResult<A>;
}

/// A temporary "splice" to add, update, delete and monitor elements in a sequence of elements."
/// It is mainly intended for view sequences
///
/// Usually it's backed by a collection (e.g. `Vec`) that holds all the (existing) elements.
/// It sweeps over the element collection and does updates in place.
/// Internally it works by having a pointer/index to the current/old element (0 at the beginning),
/// and the pointer is incremented by basically all methods that mutate that sequence.
pub trait ElementsSplice {
    /// Insert a new element at the current index in the resulting collection (and increment the index by 1)
    fn push(&mut self, element: Box<dyn Widget>, cx: &mut Cx);

    /// Mutate the next existing element, and add it to the resulting collection (and increment the index by 1)
    fn mutate(&mut self, cx: &mut Cx) -> &mut WidgetMut<Box<dyn Widget>>;

    /// Mark any changes done by `mutate` on the current element (this doesn't change the index)
    fn mark(&mut self, changeflags: ChangeFlags, cx: &mut Cx) -> ChangeFlags;

    /// Delete the next n existing elements (this doesn't change the index)
    fn delete(&mut self, n: usize, cx: &mut Cx);

    /// Current length of the elements collection
    fn len(&self) -> usize;
}

#[cfg(FALSE)]
impl<'a, 'b> ElementsSplice for xilem_core::VecSplice<'a, 'b, Pod> {
    fn push(&mut self, element: Pod, _cx: &mut Cx) {
        self.push(element);
    }
    fn mutate(&mut self, _cx: &mut Cx) -> &mut Pod {
        self.mutate()
    }
    fn mark(&mut self, changeflags: ChangeFlags, _cx: &mut Cx) -> ChangeFlags {
        self.last_mutated_mut()
            .map(|pod| pod.mark(changeflags))
            .unwrap_or_default()
    }
    fn delete(&mut self, n: usize, _cx: &mut Cx) {
        self.delete(n)
    }
    fn len(&self) -> usize {
        self.len()
    }
}

/// This trait represents a (possibly empty) sequence of views.
///
/// It is up to the parent view how to lay out and display them.
pub trait ViewSequence<T, A = ()>: Send {
    /// Associated states for the views.
    type State: Send;
    /// Build the associated widgets and initialize all states.
    ///
    /// To be able to monitor changes (e.g. tree-structure tracking) rather than just adding elements,
    /// this takes an element splice as well (when it could be just a `Vec` otherwise)
    fn build(&self, cx: &mut Cx, elements: &mut dyn ElementsSplice) -> Self::State;

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        elements: &mut dyn ElementsSplice,
    ) -> ChangeFlags;

    /// Propagate a message.
    ///
    /// Handle a message, propagating to elements if needed. Here, `id_path` is a slice
    /// of ids beginning at an element of this view_sequence.
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A>;

    /// Returns the current amount of widgets built by this sequence.
    fn count(&self, state: &Self::State) -> usize;
}

impl<T, A, V: View<T, A> + ViewMarker> ViewSequence<T, A> for V
where
    V::Element: Widget + 'static,
{
    type State = (<V as View<T, A>>::State, xilem_core::Id);
    fn build(&self, cx: &mut Cx, elements: &mut dyn ElementsSplice) -> Self::State {
        let (id, state, pod) = cx.with_new_widget(|cx| <V as View<T, A>>::build(self, cx));
        elements.push(pod, cx);
        (state, id)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        elements: &mut dyn ElementsSplice,
    ) -> ChangeFlags {
        let pod = elements.mutate(cx);
        let flags = cx.with_widget(pod, |el, cx| {
            <V as View<T, A>>::rebuild(self, cx, prev, &mut state.1, &mut state.0, el)
        });
        elements.mark(flags, cx)
    }

    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        if let Some((first, rest_path)) = id_path.split_first() {
            if first == &state.1 {
                return <V as View<T, A>>::message(
                    self,
                    rest_path,
                    &mut state.0,
                    message,
                    app_state,
                );
            }
        }
        xilem_core::MessageResult::Stale(message)
    }

    fn count(&self, _state: &Self::State) -> usize {
        1
    }
}

impl<T, A, VT: ViewSequence<T, A>> ViewSequence<T, A> for Option<VT> {
    type State = Option<VT::State>;
    fn build(&self, cx: &mut Cx, elements: &mut dyn ElementsSplice) -> Self::State {
        match self {
            None => None,
            Some(vt) => {
                let state = vt.build(cx, elements);
                Some(state)
            }
        }
    }
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        elements: &mut dyn ElementsSplice,
    ) -> ChangeFlags {
        match (self, &mut *state, prev) {
            (Some(this), Some(state), Some(prev)) => this.rebuild(cx, prev, state, elements),
            (None, Some(seq_state), Some(prev)) => {
                let count = prev.count(&seq_state);
                elements.delete(count, cx);
                *state = None;
                <ChangeFlags>::tree_structure()
            }
            (Some(this), None, None) => {
                *state = Some(this.build(cx, elements));
                <ChangeFlags>::tree_structure()
            }
            (None, None, None) => <ChangeFlags>::empty(),
            _ => panic!("non matching state and prev value"),
        }
    }
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        match (self, state) {
            (Some(vt), Some(state)) => vt.message(id_path, state, message, app_state),
            (None, None) => xilem_core::MessageResult::Stale(message),
            _ => panic!("non matching state and prev value"),
        }
    }
    fn count(&self, state: &Self::State) -> usize {
        match (self, state) {
            (Some(vt), Some(state)) => vt.count(state),
            (None, None) => 0,
            _ => panic!("non matching state and prev value"),
        }
    }
}

impl<T, A, VT: ViewSequence<T, A>> ViewSequence<T, A> for Vec<VT> {
    type State = Vec<VT::State>;
    fn build(&self, cx: &mut Cx, elements: &mut dyn ElementsSplice) -> Self::State {
        self.iter().map(|child| child.build(cx, elements)).collect()
    }
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        elements: &mut dyn ElementsSplice,
    ) -> ChangeFlags {
        let mut changed = <ChangeFlags>::default();
        for ((child, child_prev), child_state) in self.iter().zip(prev).zip(state.iter_mut()) {
            let el_changed = child.rebuild(cx, child_prev, child_state, elements);
            changed |= el_changed;
        }
        let n = self.len();
        if n < prev.len() {
            let n_delete = state
                .splice(n.., [])
                .enumerate()
                .map(|(i, state)| prev[n + i].count(&state))
                .sum();
            elements.delete(n_delete, cx);
            changed |= <ChangeFlags>::tree_structure();
        } else if n > prev.len() {
            for i in prev.len()..n {
                state.push(self[i].build(cx, elements));
            }
            changed |= <ChangeFlags>::tree_structure();
        }
        changed
    }
    fn count(&self, state: &Self::State) -> usize {
        self.iter()
            .zip(state)
            .map(|(child, child_state)| child.count(child_state))
            .sum()
    }
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        let mut result = xilem_core::MessageResult::Stale(message);
        for (child, child_state) in self.iter().zip(state) {
            if let xilem_core::MessageResult::Stale(message) = result {
                result = child.message(id_path, child_state, message, app_state);
            } else {
                break;
            }
        }
        result
    }
}

/// This trait marks a type a View.
///
/// This trait is a workaround for Rust's orphan rules. It serves as a switch between
/// default and custom
#[doc = concat!("`",stringify!(ViewSequence),"`")]
/// implementations. You can't implement
#[doc = concat!("`",stringify!(ViewSequence),"`")]
/// for types which also implement
#[doc = concat!("`",stringify!(ViewMarker),"`.")]
pub trait ViewMarker {}

xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags, ;
);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0;
0);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1;
0,1);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2;
0,1,2);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2,V3;
0,1,2,3);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2,V3,V4;
0,1,2,3,4);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2,V3,V4,V5;
0,1,2,3,4,5);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2,V3,V4,V5,V6;
0,1,2,3,4,5,6);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2,V3,V4,V5,V6,V7;
0,1,2,3,4,5,6,7);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2,V3,V4,V5,V6,V7,V8;
0,1,2,3,4,5,6,7,8);
xilem_core::impl_view_tuple!(ViewSequence,ElementsSplice,Pod,Cx,ChangeFlags,V0,V1,V2,V3,V4,V5,V6,V7,V8,V9;
0,1,2,3,4,5,6,7,8,9);

/// A trait enabling type erasure of views.
pub trait AnyView<T, A = ()> {
    fn as_any(&self) -> &dyn std::any::Any;

    fn dyn_build(
        &self,
        cx: &mut Cx,
    ) -> (
        xilem_core::Id,
        Box<dyn std::any::Any + Send>,
        Box<dyn Widget>,
    );

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, A>,
        id: &mut xilem_core::Id,
        state: &mut Box<dyn std::any::Any + Send>,
        element: &mut Box<dyn Widget>,
    ) -> ChangeFlags;

    fn dyn_message(
        &self,
        id_path: &[WidgetId],
        state: &mut dyn std::any::Any,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A>;
}

impl<T, A, V: View<T, A> + 'static> AnyView<T, A> for V
where
    V::State: 'static,
    V::Element: Widget + 'static,
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn dyn_build(
        &self,
        cx: &mut Cx,
    ) -> (
        xilem_core::Id,
        Box<dyn std::any::Any + Send>,
        Box<dyn Widget>,
    ) {
        let (id, state, element) = self.build(cx);
        (id, Box::new(state), Box::new(element))
    }
    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, A>,
        id: &mut xilem_core::Id,
        state: &mut Box<dyn std::any::Any + Send>,
        element: &mut Box<dyn Widget>,
    ) -> ChangeFlags {
        use std::ops::DerefMut;
        if let Some(prev) = prev.as_any().downcast_ref() {
            if let Some(state) = state.downcast_mut() {
                if let Some(element) = element.deref_mut().as_any_mut().downcast_mut() {
                    self.rebuild(cx, prev, id, state, element)
                } else {
                    eprintln!("downcast of element failed in dyn_rebuild");
                    <ChangeFlags>::default()
                }
            } else {
                eprintln!("downcast of state failed in dyn_rebuild");
                <ChangeFlags>::default()
            }
        } else {
            let (new_id, new_state, new_element) = self.build(cx);
            *id = new_id;
            *state = Box::new(new_state);
            *element = Box::new(new_element);
            <ChangeFlags>::tree_structure()
        }
    }
    fn dyn_message(
        &self,
        id_path: &[WidgetId],
        state: &mut dyn std::any::Any,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        if let Some(state) = state.downcast_mut() {
            self.message(id_path, state, message, app_state)
        } else {
            panic!("downcast error in dyn_event");
        }
    }
}
pub type BoxedView<T, A = ()> = Box<dyn AnyView<T, A> + Send>;

impl<T, A> ViewMarker for BoxedView<T, A> {}

impl<T, A> View<T, A> for BoxedView<T, A> {
    type State = Box<dyn std::any::Any + Send>;
    type Element = Box<dyn Widget>;
    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        use std::ops::Deref;
        self.deref().dyn_build(cx)
    }
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut WidgetMut<Self::Element>,
    ) -> ChangeFlags {
        use std::ops::Deref;
        self.deref()
            .dyn_rebuild(cx, prev.deref(), id, state, element)
    }
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        use std::ops::{Deref, DerefMut};
        self.deref()
            .dyn_message(id_path, state.deref_mut(), message, app_state)
    }
}

pub struct Memoize<D, F> {
    data: D,
    child_cb: F,
}
pub struct MemoizeState<T, A, V: View<T, A>> {
    view: V,
    view_state: V::State,
    dirty: bool,
}

impl<D, V, F> Memoize<D, F>
where
    F: Fn(&D) -> V,
{
    pub fn new(data: D, child_cb: F) -> Self {
        Memoize { data, child_cb }
    }
}

impl<D, F> ViewMarker for Memoize<D, F> {}

impl<T, A, D, V, F> View<T, A> for Memoize<D, F>
where
    D: PartialEq + Send + 'static,
    V: View<T, A>,
    F: Fn(&D) -> V + Send,
{
    type State = MemoizeState<T, A, V>;
    type Element = V::Element;
    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        let view = (self.child_cb)(&self.data);
        let (id, view_state, element) = view.build(cx);
        let memoize_state = MemoizeState {
            view,
            view_state,
            dirty: false,
        };
        (id, memoize_state, element)
    }
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut WidgetMut<Self::Element>,
    ) -> ChangeFlags {
        if std::mem::take(&mut state.dirty) || prev.data != self.data {
            let view = (self.child_cb)(&self.data);
            let changed = view.rebuild(cx, &state.view, id, &mut state.view_state, element);
            state.view = view;
            changed
        } else {
            <ChangeFlags>::empty()
        }
    }
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        event: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> xilem_core::MessageResult<A> {
        let r = state
            .view
            .message(id_path, &mut state.view_state, event, app_state);
        if matches!(r, xilem_core::MessageResult::RequestRebuild) {
            state.dirty = true;
        }
        r
    }
}

/// A static view, all of the content of the `view` should be constant, as this function is only run once
pub fn s<V, F>(view: F) -> Memoize<(), impl Fn(&()) -> V>
where
    F: Fn() -> V + Send + 'static,
{
    Memoize::new((), move |_: &()| view())
}
/// Memoize the view, until the `data` changes (in which case `view` is called again)
pub fn memoize<D, V, F>(data: D, view: F) -> Memoize<D, F>
where
    F: Fn(&D) -> V + Send,
{
    Memoize::new(data, view)
}

/// A view that wraps a child view and modifies the state that callbacks have access to.
///
/// # Examples
///
/// Suppose you have an outer type that looks like
///
/// ```ignore
/// struct State {
///     todos: Vec<Todo>
/// }
/// ```
///
/// and an inner type/view that looks like
///
/// ```ignore
/// struct Todo {
///     label: String
/// }
///
/// struct TodoView {
///     label: String
/// }
///
/// enum TodoAction {
///     Delete
/// }
///
/// impl View<Todo, TodoAction> for TodoView {
///     // ...
/// }
/// ```
///
/// then your top-level action (`()`) and state type (`State`) don't match `TodoView`'s.
/// You can use the `Adapt` view to mediate between them:
///
/// ```ignore
/// state
///     .todos
///     .enumerate()
///     .map(|(idx, todo)| {
///         Adapt::new(
///             move |data: &mut AppState, thunk| {
///                 if let MessageResult::Action(action) = thunk.call(&mut data.todos[idx]) {
///                     match action {
///                         TodoAction::Delete => data.todos.remove(idx),
///                     }
///                 }
///                 MessageResult::Nop
///             },
///             TodoView { label: todo.label }
///         )
///     })
/// ```
pub struct Adapt<
    ParentT,
    ParentA,
    ChildT,
    ChildA,
    V,
    F = fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> xilem_core::MessageResult<ParentA>,
> {
    f: F,
    child: V,
    phantom: std::marker::PhantomData<fn() -> (ParentT, ParentA, ChildT, ChildA)>,
}
/// A "thunk" which dispatches an message to an adapt node's child.
///
/// The closure passed to [`Adapt`][crate::Adapt] should call this thunk with the child's
/// app state.
pub struct AdaptThunk<'a, ChildT, ChildA, V: View<ChildT, ChildA>> {
    child: &'a V,
    state: &'a mut V::State,
    id_path: &'a [xilem_core::Id],
    message: Box<dyn std::any::Any>,
}

impl<ParentT, ParentA, ChildT, ChildA, V, F> Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
where
    V: View<ChildT, ChildA>,
    F: Fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> xilem_core::MessageResult<ParentA> + Send,
{
    pub fn new(f: F, child: V) -> Self {
        Adapt {
            f,
            child,
            phantom: Default::default(),
        }
    }
}

impl<'a, ChildT, ChildA, V: View<ChildT, ChildA>> AdaptThunk<'a, ChildT, ChildA, V> {
    pub fn call(self, app_state: &mut ChildT) -> xilem_core::MessageResult<ChildA> {
        self.child
            .message(self.id_path, self.state, self.message, app_state)
    }
}

impl<ParentT, ParentA, ChildT, ChildA, V, F> View<ParentT, ParentA>
    for Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
where
    V: View<ChildT, ChildA>,
    F: Fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> xilem_core::MessageResult<ParentA> + Send,
{
    type State = V::State;
    type Element = V::Element;
    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        self.child.build(cx)
    }
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut WidgetMut<Self::Element>,
    ) -> ChangeFlags {
        self.child.rebuild(cx, &prev.child, id, state, element)
    }
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut ParentT,
    ) -> xilem_core::MessageResult<ParentA> {
        let thunk = AdaptThunk {
            child: &self.child,
            state,
            id_path,
            message,
        };
        (self.f)(app_state, thunk)
    }
}

impl<ParentT, ParentA, ChildT, ChildA, V, F> ViewMarker
    for Adapt<ParentT, ParentA, ChildT, ChildA, V, F>
where
    V: View<ChildT, ChildA>,
    F: Fn(&mut ParentT, AdaptThunk<ChildT, ChildA, V>) -> xilem_core::MessageResult<ParentA> + Send,
{
}

/// A view that wraps a child view and modifies the state that callbacks have access to.
pub struct AdaptState<ParentT, ChildT, V, F = fn(&mut ParentT) -> &mut ChildT> {
    f: F,
    child: V,
    phantom: std::marker::PhantomData<fn() -> (ParentT, ChildT)>,
}

impl<ParentT, ChildT, V, F> AdaptState<ParentT, ChildT, V, F>
where
    F: Fn(&mut ParentT) -> &mut ChildT + Send,
{
    pub fn new(f: F, child: V) -> Self {
        Self {
            f,
            child,
            phantom: Default::default(),
        }
    }
}

impl<ParentT, ChildT, A, V, F> View<ParentT, A> for AdaptState<ParentT, ChildT, V, F>
where
    V: View<ChildT, A>,
    F: Fn(&mut ParentT) -> &mut ChildT + Send,
{
    type State = V::State;
    type Element = V::Element;
    fn build(&self, cx: &mut Cx) -> (xilem_core::Id, Self::State, Self::Element) {
        self.child.build(cx)
    }
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut xilem_core::Id,
        state: &mut Self::State,
        element: &mut WidgetMut<Self::Element>,
    ) -> ChangeFlags {
        self.child.rebuild(cx, &prev.child, id, state, element)
    }
    fn message(
        &self,
        id_path: &[WidgetId],
        state: &mut Self::State,
        message: Box<dyn std::any::Any>,
        app_state: &mut ParentT,
    ) -> xilem_core::MessageResult<A> {
        self.child
            .message(id_path, state, message, (self.f)(app_state))
    }
}

impl<ParentT, ChildT, V, F> ViewMarker for AdaptState<ParentT, ChildT, V, F> where
    F: Fn(&mut ParentT) -> &mut ChildT + Send
{
}

#[derive(Clone)]
pub struct Cx {
    id_path: IdPath,
    element_id_path: Vec<WidgetId>, // Note that this is the widget id type.
    req_chan: SyncSender<IdPath>,
    pub(crate) tree_structure: TreeStructure,
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
            element_id_path: Vec::new(),
            req_chan: req_chan.clone(),
            pending_async: HashSet::new(),
            tree_structure: TreeStructure::default(),
        }
    }

    pub fn push(&mut self, id: Id) {
        self.id_path.push(id);
    }

    pub fn pop(&mut self) {
        self.id_path.pop();
    }

    pub fn id_path_is_empty(&self) -> bool {
        self.id_path.is_empty()
    }

    pub fn id_path(&self) -> &IdPath {
        &self.id_path
    }

    pub fn element_id_path_is_empty(&self) -> bool {
        self.element_id_path.is_empty()
    }

    /// Return the element id of the current element/widget
    pub fn element_id(&self) -> WidgetId {
        *self
            .element_id_path
            .last()
            .expect("element_id path imbalance, there should be an element id")
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
    pub fn with_new_widget<S, E, F>(&mut self, f: F) -> (Id, S, WidgetPod<E>)
    where
        E: Widget + 'static,
        F: FnOnce(&mut Cx) -> (Id, S, E),
    {
        let pod_id = WidgetId::next();
        self.element_id_path.push(pod_id);
        let (id, state, element) = f(self);
        self.element_id_path.pop();
        (id, state, WidgetPod::new_with_id(element, pod_id))
    }

    /// Run some logic within the context of a given Pod,
    ///
    /// This logic is usually `View::rebuild`
    ///
    /// # Panics
    ///
    /// When the element type `E` is not the same type as the inner `Widget` of the `Pod`.
    pub fn with_widget<T, E, F>(&mut self, widget: &mut WidgetMut<'_, E>, f: F) -> T
    where
        E: Widget + StoreInWidgetMut + 'static,
        F: FnOnce(&mut WidgetMut<E>, &mut Cx) -> T,
    {
        self.element_id_path.push(widget.id());
        let result = f(widget, self);
        self.element_id_path.pop();
        result
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
