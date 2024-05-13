// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use masonry::{widget::WidgetMut, WidgetPod};

use crate::{MasonryView, MessageResult, ViewCx, ViewId};

pub struct Memoize<D, F> {
    data: D,
    child_cb: F,
}

pub struct MemoizeState<T, A, V: MasonryView<T, A>> {
    view: V,
    view_state: V::ViewState,
    dirty: bool,
}

impl<D, V, F> Memoize<D, F>
where
    F: Fn(&D) -> V,
{
    const ASSERT_CONTEXTLESS_FN: () = {
        assert!(
            std::mem::size_of::<F>() == 0,
            "
It's not possible to use function pointers or captured context in closures,
as this potentially messes up the logic of memoize or produces unwanted effects.

For example a different kind of view could be instantiated with a different callback, while the old one is still memoized, but it's not updated then.
It's not possible in Rust currently to check whether the (content of the) callback has changed with the `Fn` trait, which would make this otherwise possible.
"
        );
    };

    pub fn new(data: D, child_cb: F) -> Self {
        #[allow(clippy::let_unit_value)]
        let _ = Self::ASSERT_CONTEXTLESS_FN;
        Memoize { data, child_cb }
    }
}

impl<State, Action, D, V, F> MasonryView<State, Action> for Memoize<D, F>
where
    D: PartialEq + Send + Sync + 'static,
    V: MasonryView<State, Action>,
    F: Fn(&D) -> V + Send + Sync + 'static,
{
    type ViewState = MemoizeState<State, Action, V>;

    type Element = V::Element;

    fn build(&self, cx: &mut ViewCx) -> (WidgetPod<Self::Element>, Self::ViewState) {
        let view = (self.child_cb)(&self.data);
        let (element, view_state) = view.build(cx);
        let memoize_state = MemoizeState {
            view,
            view_state,
            dirty: false,
        };
        (element, memoize_state)
    }

    fn rebuild(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut ViewCx,
        prev: &Self,
        element: WidgetMut<Self::Element>,
    ) {
        if std::mem::take(&mut view_state.dirty) || prev.data != self.data {
            let view = (self.child_cb)(&self.data);
            view.rebuild(&mut view_state.view_state, cx, &view_state.view, element);
            view_state.view = view;
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: Box<dyn Any>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let r = view_state
            .view
            .message(&mut view_state.view_state, id_path, message, app_state);
        if matches!(r, MessageResult::RequestRebuild) {
            view_state.dirty = true;
        }
        r
    }
}

/// A static view, all of the content of the `view` should be constant, as this function is only run once
pub fn static_view<V, F>(view: F) -> Memoize<(), impl Fn(&()) -> V>
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
