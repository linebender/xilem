// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{FromDynWidget, Widget};
use masonry::widgets::Passthrough;

use crate::core::{AnyElement, AnyView, Mut, SuperElement};
use crate::{Pod, ViewCtx};

/// A view which can have any underlying view type.
///
/// This can be used to return type erased views (such as from a trait),
/// or used to implement conditional display and switching of views.
///
/// Note that `Option` can also be used for conditionally displaying
/// views in a [`ViewSequence`](xilem_core::ViewSequence).
// TODO: Mention `Either` when we have implemented that?
pub type AnyWidgetView<State, Action = ()> =
    dyn AnyView<State, Action, ViewCtx, Pod<Passthrough>> + Send + Sync;

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for Pod<Passthrough> {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        ctx.create_pod(Passthrough::new(child.new_widget.erased()))
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = {
            let mut child = Passthrough::child_mut(&mut this);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> AnyElement<Pod<W>, ViewCtx> for Pod<Passthrough> {
    fn replace_inner(mut this: Self::Mut<'_>, child: Pod<W>) -> Self::Mut<'_> {
        Passthrough::set_child(&mut this, child.new_widget.erased());
        this
    }
}
