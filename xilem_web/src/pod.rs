// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::ops::DerefMut as _;

use crate::core::{AnyElement, SuperElement, ViewElement};
use crate::{AnyNode, DomNode, ViewCtx};
use wasm_bindgen::UnwrapThrowExt;

/// A container, which holds the actual DOM node, and associated props, such as attributes or classes.
///
/// These attributes are not directly set on the DOM node to avoid mutating or reading from the DOM tree unnecessarily, and to have more control over the whole update flow.
pub struct Pod<N: DomNode> {
    pub node: N,
    pub flags: PodFlags,
    pub props: N::Props,
}

/// Type-erased [`Pod`], it's used for example as intermediate representation for children of a DOM node
pub type AnyPod = Pod<Box<dyn AnyNode>>;

impl<N: DomNode> Pod<N> {
    pub const fn new(node: N, props: N::Props, flags: PodFlags) -> Self {
        Pod { node, props, flags }
    }

    /// Erases the type of this [`Pod`] and applies props if necessary.
    pub fn into_any_pod(mut pod: Pod<N>) -> AnyPod {
        pod.apply_changes();
        Pod {
            node: Box::new(pod.node),
            props: Box::new(pod.props),
            flags: pod.flags,
        }
    }

    /// Applies props and cleans flags.
    pub(crate) fn apply_changes(&mut self) {
        if self.flags.needs_update() {
            self.node.apply_props(&mut self.props, &mut self.flags);
        }
        self.flags.clear();
    }
}

impl AnyPod {
    pub(crate) fn as_mut<'a>(
        &'a mut self,
        parent: impl Into<Option<&'a web_sys::Node>>,
        was_removed: bool,
    ) -> PodMut<'a, Box<dyn AnyNode>> {
        PodMut::new(
            &mut self.node,
            &mut self.props,
            &mut self.flags,
            parent.into(),
            was_removed,
        )
    }
}

impl<N: DomNode> ViewElement for Pod<N> {
    type Mut<'a> = PodMut<'a, N>;
}

impl<N: DomNode> SuperElement<Pod<N>, ViewCtx> for AnyPod {
    fn upcast(_ctx: &mut ViewCtx, child: Pod<N>) -> Self {
        Pod::into_any_pod(child)
    }

    fn with_downcast_val<R>(
        mut this: Self::Mut<'_>,
        f: impl FnOnce(PodMut<N>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let downcast = this.downcast();
        let ret = f(downcast);
        (this, ret)
    }
}

impl<N: DomNode> AnyElement<Pod<N>, ViewCtx> for AnyPod {
    fn replace_inner(this: Self::Mut<'_>, mut child: Pod<N>) -> Self::Mut<'_> {
        child.apply_changes();
        if let Some(parent) = this.parent {
            parent
                .replace_child(child.node.as_ref(), this.node.as_ref())
                .unwrap_throw();
        }
        *this.node = Box::new(child.node);
        *this.props = Box::new(child.props);
        *this.flags = child.flags;
        this
    }
}

/// The mutable representation of [`Pod`].
///
/// This is a container which contains info of what has changed and provides mutable access to the underlying element and its props
/// When it's dropped all changes are applied to the underlying DOM node
pub struct PodMut<'a, N: DomNode> {
    pub node: &'a mut N,
    pub props: &'a mut N::Props,
    pub flags: &'a mut PodFlags,
    pub parent: Option<&'a web_sys::Node>,
    pub was_removed: bool,
    pub is_reborrow: bool,
}

impl<'a, N: DomNode> PodMut<'a, N> {
    pub fn new(
        node: &'a mut N,
        props: &'a mut N::Props,
        flags: &'a mut PodFlags,
        parent: Option<&'a web_sys::Node>,
        was_removed: bool,
    ) -> PodMut<'a, N> {
        PodMut {
            node,
            props,
            flags,
            parent,
            was_removed,
            is_reborrow: false,
        }
    }

    pub fn reborrow_mut(&mut self) -> PodMut<N> {
        PodMut {
            node: self.node,
            props: self.props,
            flags: self.flags,
            parent: self.parent,
            was_removed: self.was_removed,
            is_reborrow: true,
        }
    }

    pub(crate) fn apply_changes(&mut self) {
        if self.flags.needs_update() {
            self.node.apply_props(self.props, self.flags);
        }
        self.flags.clear();
    }
}

impl PodMut<'_, Box<dyn AnyNode>> {
    fn downcast<N: DomNode>(&mut self) -> PodMut<N> {
        PodMut::new(
            self.node.deref_mut().as_any_mut().downcast_mut().unwrap(),
            self.props.downcast_mut().unwrap(),
            self.flags,
            self.parent,
            false,
        )
    }
}

impl<N: DomNode> Drop for PodMut<'_, N> {
    fn drop(&mut self) {
        if self.is_reborrow || self.was_removed {
            return;
        }
        self.apply_changes();
    }
}

impl<T, N: AsRef<T> + DomNode> AsRef<T> for Pod<N> {
    fn as_ref(&self) -> &T {
        <N as AsRef<T>>::as_ref(&self.node)
    }
}

impl<T, N: AsRef<T> + DomNode> AsRef<T> for PodMut<'_, N> {
    fn as_ref(&self) -> &T {
        <N as AsRef<T>>::as_ref(self.node)
    }
}

// TODO maybe use bitflags for this, but not sure if it's worth it to pull the dependency in just for this.
/// General flags describing the current state of the element (in hydration, was created, needs update (in general for optimization))
pub struct PodFlags(u8);

impl PodFlags {
    const IN_HYDRATION: u8 = 1 << 0;
    const WAS_CREATED: u8 = 1 << 1;
    const NEEDS_UPDATE: u8 = 1 << 2;

    pub(crate) fn new(in_hydration: bool) -> Self {
        if in_hydration {
            PodFlags(Self::WAS_CREATED | Self::IN_HYDRATION)
        } else {
            PodFlags(Self::WAS_CREATED)
        }
    }

    /// This should only be used in tests, other than within the [`Element`] props
    pub(crate) fn clear(&mut self) {
        self.0 = 0;
    }

    /// Whether the current element was just created, this is usually `true` within `View::build`, but can also happen, e.g. within a `OneOf` variant change.
    pub fn was_created(&self) -> bool {
        self.0 & Self::WAS_CREATED != 0
    }

    /// Whether the current element is within a hydration context, that could e.g. happen when inside a [`Templated`](crate::Templated) view.
    pub fn in_hydration(&self) -> bool {
        self.0 & Self::IN_HYDRATION != 0
    }

    /// Whether the current element generally needs to be updated, this serves as cheap preliminary check whether anything changed at all.
    pub fn needs_update(&self) -> bool {
        self.0 & Self::NEEDS_UPDATE != 0
    }

    /// This should be called as soon as anything has changed for the current element (except children, as they're handled within the element views).
    pub fn set_needs_update(&mut self) {
        self.0 |= Self::NEEDS_UPDATE;
    }
}
