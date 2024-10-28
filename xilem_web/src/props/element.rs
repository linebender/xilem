// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    document,
    events::Events,
    modifiers::{Attributes, Children, Classes, Modifier, Styles, WithModifier},
    AnyPod, Pod, ViewCtx,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;

// TODO maybe use bitflags for this, but not sure if it's worth it to pull the dependency in just for this.
/// General flags describing the current state of the element (in hydration, was created, needs update (in general for optimization))
pub struct ElementFlags(u8);

impl ElementFlags {
    const IN_HYDRATION: u8 = 1 << 0;
    const WAS_CREATED: u8 = 1 << 1;
    const NEEDS_UPDATE: u8 = 1 << 2;

    pub(crate) fn new(in_hydration: bool) -> Self {
        if in_hydration {
            ElementFlags(Self::WAS_CREATED | Self::IN_HYDRATION)
        } else {
            ElementFlags(Self::WAS_CREATED)
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

// Lazy access to attributes etc. to avoid allocating unnecessary memory when it isn't needed
// Benchmarks have shown, that this can significantly increase performance and reduce memory usage...
/// This holds all the state for a DOM [`Element`](`crate::interfaces::Element`), it is used for [`DomNode::Props`](`crate::DomNode::Props`)
pub struct Element {
    pub(crate) flags: ElementFlags,
    pub(crate) attributes: Option<Box<Attributes>>,
    pub(crate) classes: Option<Box<Classes>>,
    pub(crate) styles: Option<Box<Styles>>,
    pub(crate) children: Vec<AnyPod>,
    pub(crate) events: Events,
}

impl Element {
    pub fn new(
        children: Vec<AnyPod>,
        attr_size_hint: usize,
        style_size_hint: usize,
        class_size_hint: usize,
        in_hydration: bool,
    ) -> Self {
        Self {
            attributes: (attr_size_hint > 0).then(|| Attributes::new(attr_size_hint).into()),
            classes: (class_size_hint > 0).then(|| Classes::new(class_size_hint).into()),
            styles: (style_size_hint > 0).then(|| Styles::new(style_size_hint).into()),
            children,
            flags: ElementFlags::new(in_hydration),
            events: Events,
        }
    }

    // All of this is slightly more complicated than it should be,
    // because we want to minimize DOM traffic as much as possible (that's basically the bottleneck)
    pub fn update_element(&mut self, element: &web_sys::Element) {
        if self.flags.needs_update() {
            if let Some(attributes) = &mut self.attributes {
                Attributes::apply_changes(Modifier::new(attributes, &mut self.flags), element);
            }
            if let Some(classes) = &mut self.classes {
                Classes::apply_changes(Modifier::new(classes, &mut self.flags), element);
            }
            if let Some(styles) = &mut self.styles {
                Styles::apply_changes(Modifier::new(styles, &mut self.flags), element);
            }
        }
        self.flags.clear();
    }
}

impl Pod<web_sys::Element> {
    /// Creates a new Pod with [`web_sys::Element`] as element and [`Element`] as its [`DomNode::Props`](`crate::DomNode::Props`).
    pub fn new_element_with_ctx(
        children: Vec<AnyPod>,
        ns: &str,
        elem_name: &str,
        ctx: &mut ViewCtx,
    ) -> Self {
        let attr_size_hint = ctx.take_modifier_size_hint::<Attributes>();
        let class_size_hint = ctx.take_modifier_size_hint::<Classes>();
        let style_size_hint = ctx.take_modifier_size_hint::<Styles>();
        let element = document()
            .create_element_ns(
                Some(wasm_bindgen::intern(ns)),
                wasm_bindgen::intern(elem_name),
            )
            .unwrap_throw();

        for child in children.iter() {
            let _ = element.append_child(child.node.as_ref());
        }

        Self {
            node: element,
            props: Element::new(
                children,
                attr_size_hint,
                style_size_hint,
                class_size_hint,
                false,
            ),
        }
    }

    /// Creates a new Pod that hydrates an existing node (within the `ViewCtx`) as [`web_sys::Element`] and [`Element`] as its [`DomNode::Props`](`crate::DomNode::Props`).
    pub fn hydrate_element_with_ctx(children: Vec<AnyPod>, ctx: &mut ViewCtx) -> Self {
        let attr_size_hint = ctx.take_modifier_size_hint::<Attributes>();
        let class_size_hint = ctx.take_modifier_size_hint::<Classes>();
        let style_size_hint = ctx.take_modifier_size_hint::<Styles>();
        let element = ctx.hydrate_node().unwrap_throw();

        Self {
            node: element.unchecked_into(),
            props: Element::new(
                children,
                attr_size_hint,
                style_size_hint,
                class_size_hint,
                true,
            ),
        }
    }
}

impl WithModifier<Attributes> for Element {
    fn modifier(&mut self) -> Modifier<'_, Attributes> {
        let modifier = self
            .attributes
            .get_or_insert_with(|| Attributes::new(0).into());
        Modifier::new(modifier, &mut self.flags)
    }
}

impl WithModifier<Children> for Element {
    fn modifier(&mut self) -> Modifier<'_, Children> {
        Modifier::new(&mut self.children, &mut self.flags)
    }
}

impl WithModifier<Classes> for Element {
    fn modifier(&mut self) -> Modifier<'_, Classes> {
        let modifier = self.classes.get_or_insert_with(|| Classes::new(0).into());
        Modifier::new(modifier, &mut self.flags)
    }
}

impl WithModifier<Events> for Element {
    fn modifier(&mut self) -> Modifier<'_, Events> {
        Modifier::new(&mut self.events, &mut self.flags)
    }
}

impl WithModifier<Styles> for Element {
    fn modifier(&mut self) -> Modifier<'_, Styles> {
        let modifier = self.styles.get_or_insert_with(|| Styles::new(0).into());
        Modifier::new(modifier, &mut self.flags)
    }
}

/// An alias trait to sum up all modifiers that a DOM `Element` can have. It's used to avoid a lot of boilerplate in public APIs.
pub trait WithElementProps:
    WithModifier<Attributes> + WithModifier<Children> + WithModifier<Classes> + WithModifier<Styles>
{
}
impl<
        T: WithModifier<Attributes>
            + WithModifier<Children>
            + WithModifier<Classes>
            + WithModifier<Styles>,
    > WithElementProps for T
{
}
