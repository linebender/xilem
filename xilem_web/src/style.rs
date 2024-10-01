// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use xilem_core::{MessageResult, Mut, View, ViewElement, ViewId, ViewMarker};

use crate::{vecmap::VecMap, DomNode, DynMessage, ElementProps, Pod, PodMut, ViewCtx};

type CowStr = std::borrow::Cow<'static, str>;

/// A trait to make the class adding functions generic over collection type
pub trait IntoStyles {
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>);
}

struct StyleTuple<T1, T2>(T1, T2);

// TODO should this also allow removing style values, via `None`?
/// Create a style from a style name and its value.
pub fn style(name: impl Into<CowStr>, value: impl Into<CowStr>) -> impl IntoStyles {
    StyleTuple(name, Some(value.into()))
}

impl<T1, T2> IntoStyles for StyleTuple<T1, T2>
where
    T1: Into<CowStr>,
    T2: Into<Option<CowStr>>,
{
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>) {
        let StyleTuple(key, value) = self;
        styles.push((key.into(), value.into()));
    }
}

impl<T> IntoStyles for Option<T>
where
    T: IntoStyles,
{
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>) {
        if let Some(t) = self {
            t.into_styles(styles);
        }
    }
}

impl<T> IntoStyles for Vec<T>
where
    T: IntoStyles,
{
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>) {
        for itm in self {
            itm.into_styles(styles);
        }
    }
}

impl<T: IntoStyles, const N: usize> IntoStyles for [T; N] {
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>) {
        for itm in self {
            itm.into_styles(styles);
        }
    }
}

impl<T1, T2, S> IntoStyles for HashMap<T1, T2, S>
where
    T1: Into<CowStr>,
    T2: Into<Option<CowStr>>,
{
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>) {
        for (key, value) in self {
            styles.push((key.into(), value.into()));
        }
    }
}

impl<T1, T2> IntoStyles for BTreeMap<T1, T2>
where
    T1: Into<CowStr>,
    T2: Into<Option<CowStr>>,
{
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>) {
        for (key, value) in self {
            styles.push((key.into(), value.into()));
        }
    }
}

impl<T1, T2> IntoStyles for VecMap<T1, T2>
where
    T1: Into<CowStr>,
    T2: Into<Option<CowStr>>,
{
    fn into_styles(self, styles: &mut Vec<(CowStr, Option<CowStr>)>) {
        for (key, value) in self {
            styles.push((key.into(), value.into()));
        }
    }
}

/// This trait allows (modifying) the `style` property of `HTMLElement`/`SVGElement`s
///
/// It's e.g. used in the DOM interface traits [`HtmlElement`](`crate::interfaces::HtmlElement`) and [`SvgElement`](`crate::interfaces::SvgElement`).
/// Modifications have to be done on the up-traversal of [`View::rebuild`], i.e. after [`View::rebuild`] was invoked for descendent views.
/// See [`Style::build`] and [`Style::rebuild`], how to use this for [`ViewElement`]s that implement this trait.
/// When these methods are used, they have to be used in every reconciliation pass (i.e. [`View::rebuild`]).
pub trait WithStyle {
    /// Needs to be invoked within a [`View::rebuild`] before traversing to descendent views, and before any modifications (with [`set_style`](`WithStyle::set_style`)) are done in that view
    fn rebuild_style_modifier(&mut self);

    /// Needs to be invoked after any modifications are done
    fn mark_end_of_style_modifier(&mut self);

    /// Sets or removes (when value is `None`) a style property from the underlying element.
    ///
    /// When in [`View::rebuild`] this has to be invoked *after* traversing the inner `View` with [`View::rebuild`]
    fn set_style(&mut self, name: &CowStr, value: &Option<CowStr>);

    // TODO first find a use-case for this...
    // fn get_style(&self, name: &str) -> Option<&CowStr>;
}

#[derive(Debug, PartialEq)]
enum StyleModifier {
    Remove(CowStr),
    Set(CowStr, CowStr),
    EndMarker(u16),
}

const HYDRATING: u16 = 1 << 14;
const CREATING: u16 = 1 << 15;
const RESERVED_BIT_MASK: u16 = HYDRATING | CREATING;

#[derive(Debug, Default)]
/// This contains all the current style properties of an [`HtmlElement`](`crate::interfaces::Element`) or [`SvgElement`](`crate::interfaces::SvgElement`).
pub struct Styles {
    style_modifiers: Vec<StyleModifier>,
    updated_styles: VecMap<CowStr, ()>,
    idx: u16,
    /// the two most significant bits are reserved for whether this was just created (bit 15) and if it's currently being hydrated (bit 14)
    start_idx: u16,
}

impl Styles {
    pub(crate) fn new(size_hint: usize, #[cfg(feature = "hydration")] in_hydration: bool) -> Self {
        #[allow(unused_mut)]
        let mut start_idx = CREATING;
        #[cfg(feature = "hydration")]
        if in_hydration {
            start_idx |= HYDRATING;
        }

        Self {
            style_modifiers: Vec::with_capacity(size_hint),
            start_idx,
            ..Default::default()
        }
    }
}

fn set_style(element: &web_sys::Element, name: &str, value: &str) {
    if let Some(el) = element.dyn_ref::<web_sys::HtmlElement>() {
        el.style().set_property(name, value).unwrap_throw();
    } else if let Some(el) = element.dyn_ref::<web_sys::SvgElement>() {
        el.style().set_property(name, value).unwrap_throw();
    }
}

fn remove_style(element: &web_sys::Element, name: &str) {
    if let Some(el) = element.dyn_ref::<web_sys::HtmlElement>() {
        el.style().remove_property(name).unwrap_throw();
    } else if let Some(el) = element.dyn_ref::<web_sys::SvgElement>() {
        el.style().remove_property(name).unwrap_throw();
    }
}

impl Styles {
    pub fn apply_style_changes(&mut self, element: &web_sys::Element) {
        if (self.start_idx & HYDRATING) == HYDRATING {
            self.start_idx &= !RESERVED_BIT_MASK;
            debug_assert!(self.updated_styles.is_empty());
            return;
        }

        if (self.start_idx & CREATING) == CREATING {
            for modifier in self.style_modifiers.iter().rev() {
                match modifier {
                    StyleModifier::Remove(name) => {
                        remove_style(element, name);
                    }
                    StyleModifier::Set(name, value) => {
                        set_style(element, name, value);
                    }
                    StyleModifier::EndMarker(_) => (),
                }
            }
            self.start_idx &= !RESERVED_BIT_MASK;
            debug_assert!(self.updated_styles.is_empty());
            return;
        }

        if !self.updated_styles.is_empty() {
            for modifier in self.style_modifiers.iter().rev() {
                match modifier {
                    StyleModifier::Remove(name) => {
                        if self.updated_styles.remove(name).is_some() {
                            remove_style(element, name);
                        }
                    }
                    StyleModifier::Set(name, value) => {
                        if self.updated_styles.remove(name).is_some() {
                            set_style(element, name, value);
                        }
                    }
                    StyleModifier::EndMarker(_) => (),
                }
            }
            debug_assert!(self.updated_styles.is_empty());
        }
    }
}

impl WithStyle for Styles {
    fn set_style(&mut self, name: &CowStr, value: &Option<CowStr>) {
        if (self.start_idx & RESERVED_BIT_MASK) != 0 {
            let modifier = if let Some(value) = value {
                StyleModifier::Set(name.clone(), value.clone())
            } else {
                StyleModifier::Remove(name.clone())
            };
            self.style_modifiers.push(modifier);
        } else if let Some(modifier) = self.style_modifiers.get_mut(self.idx as usize) {
            let dirty = match (&modifier, value) {
                // early return if nothing has changed, avoids allocations
                (StyleModifier::Set(old_name, old_value), Some(new_value)) if old_name == name => {
                    if old_value == new_value {
                        false
                    } else {
                        self.updated_styles.insert(name.clone(), ());
                        true
                    }
                }
                (StyleModifier::Remove(removed), None) if removed == name => false,
                (StyleModifier::Set(old_name, _), None)
                | (StyleModifier::Remove(old_name), Some(_))
                    if old_name == name =>
                {
                    self.updated_styles.insert(name.clone(), ());
                    true
                }
                (StyleModifier::EndMarker(_), None) | (StyleModifier::EndMarker(_), Some(_)) => {
                    self.updated_styles.insert(name.clone(), ());
                    true
                }
                (StyleModifier::Set(old_name, _), _) | (StyleModifier::Remove(old_name), _) => {
                    self.updated_styles.insert(name.clone(), ());
                    self.updated_styles.insert(old_name.clone(), ());
                    true
                }
            };
            if dirty {
                *modifier = if let Some(value) = value {
                    StyleModifier::Set(name.clone(), value.clone())
                } else {
                    StyleModifier::Remove(name.clone())
                };
            }
            // else remove it out of updated_styles? (because previous styles are overwritten) not sure if worth it because potentially worse perf
        } else {
            let new_modifier = if let Some(value) = value {
                StyleModifier::Set(name.clone(), value.clone())
            } else {
                StyleModifier::Remove(name.clone())
            };
            self.updated_styles.insert(name.clone(), ());
            self.style_modifiers.push(new_modifier);
        }
        self.idx += 1;
    }

    fn rebuild_style_modifier(&mut self) {
        if self.idx == 0 {
            self.start_idx &= RESERVED_BIT_MASK;
        } else {
            let StyleModifier::EndMarker(start_idx) = self.style_modifiers[(self.idx - 1) as usize]
            else {
                unreachable!("this should not happen, as either `rebuild_style_modifier` happens first, or follows an `mark_end_of_style_modifier`")
            };
            self.idx = start_idx;
            self.start_idx = start_idx | (self.start_idx & RESERVED_BIT_MASK);
        }
    }

    fn mark_end_of_style_modifier(&mut self) {
        let start_idx = self.start_idx & !RESERVED_BIT_MASK;
        match self.style_modifiers.get_mut(self.idx as usize) {
            Some(StyleModifier::EndMarker(prev_start_idx)) if *prev_start_idx == start_idx => {} // style modifier hasn't changed
            Some(modifier) => *modifier = StyleModifier::EndMarker(start_idx),
            None => self
                .style_modifiers
                .push(StyleModifier::EndMarker(start_idx)),
        }
        self.idx += 1;
        self.start_idx = self.idx | (self.start_idx & RESERVED_BIT_MASK);
    }
}

impl WithStyle for ElementProps {
    fn rebuild_style_modifier(&mut self) {
        self.styles().rebuild_style_modifier();
    }

    fn mark_end_of_style_modifier(&mut self) {
        self.styles().mark_end_of_style_modifier();
    }

    fn set_style(&mut self, name: &CowStr, value: &Option<CowStr>) {
        self.styles().set_style(name, value);
    }
}

impl<N: DomNode> WithStyle for Pod<N>
where
    N::Props: WithStyle,
{
    fn rebuild_style_modifier(&mut self) {
        self.props.rebuild_style_modifier();
    }

    fn mark_end_of_style_modifier(&mut self) {
        self.props.mark_end_of_style_modifier();
    }

    fn set_style(&mut self, name: &CowStr, value: &Option<CowStr>) {
        self.props.set_style(name, value);
    }
}

impl<N: DomNode> WithStyle for PodMut<'_, N>
where
    N::Props: WithStyle,
{
    fn rebuild_style_modifier(&mut self) {
        self.props.rebuild_style_modifier();
    }

    fn mark_end_of_style_modifier(&mut self) {
        self.props.mark_end_of_style_modifier();
    }

    fn set_style(&mut self, name: &CowStr, value: &Option<CowStr>) {
        self.props.set_style(name, value);
    }
}

/// Syntax sugar for adding a type bound on the `ViewElement` of a view, such that both, [`ViewElement`] and [`ViewElement::Mut`] are bound to [`WithStyle`]
pub trait ElementWithStyle: for<'a> ViewElement<Mut<'a>: WithStyle> + WithStyle {}

impl<T> ElementWithStyle for T
where
    T: ViewElement + WithStyle,
    for<'a> T::Mut<'a>: WithStyle,
{
}

#[derive(Clone, Debug)]
/// A view to add `style` properties of `HTMLElement` and `SVGElement` derived elements,
pub struct Style<E, T, A> {
    el: E,
    styles: Vec<(CowStr, Option<CowStr>)>,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, T, A> Style<E, T, A> {
    pub fn new(el: E, styles: Vec<(CowStr, Option<CowStr>)>) -> Self {
        Style {
            el,
            styles,
            phantom: PhantomData,
        }
    }
}

impl<E, T, A> ViewMarker for Style<E, T, A> {}
impl<T, A, E> View<T, A, ViewCtx, DynMessage> for Style<E, T, A>
where
    T: 'static,
    A: 'static,
    E: View<T, A, ViewCtx, DynMessage, Element: ElementWithStyle>,
{
    type Element = E::Element;

    type ViewState = E::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        ctx.add_modifier_size_hint::<Styles>(self.styles.len());
        let (mut element, state) = self.el.build(ctx);
        for (key, value) in &self.styles {
            element.set_style(key, value);
        }
        element.mark_end_of_style_modifier();
        (element, state)
    }

    fn rebuild<'e>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'e, Self::Element>,
    ) -> Mut<'e, Self::Element> {
        element.rebuild_style_modifier();
        let mut element = self.el.rebuild(&prev.el, view_state, ctx, element);
        for (key, value) in &self.styles {
            element.set_style(key, value);
        }
        element.mark_end_of_style_modifier();
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.el.teardown(view_state, ctx, element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut T,
    ) -> MessageResult<A, DynMessage> {
        self.el.message(view_state, id_path, message, app_state)
    }
}
