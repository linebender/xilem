use std::marker::PhantomData;

use xilem_core::{DynMessage, MessageResult, Mut, View, ViewElement, ViewId};

// TODO maybe this attribute is redundant and can be formed just from the class_modifiers attribute
use crate::{vecmap::VecMap, DomNode, ElementProps, Pod, PodMut, ViewCtx};

type CowStr = std::borrow::Cow<'static, str>;

pub trait AsClassIter {
    fn class_iter(&self) -> impl Iterator<Item = CowStr>;
}

impl<C: AsClassIter> AsClassIter for Option<C> {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        self.iter().flat_map(|c| c.class_iter())
    }
}

impl AsClassIter for String {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        std::iter::once(self.clone().into())
    }
}

impl AsClassIter for &'static str {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        std::iter::once(CowStr::from(*self))
    }
}

impl AsClassIter for CowStr {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        std::iter::once(self.clone())
    }
}

impl<T> AsClassIter for Vec<T>
where
    T: AsClassIter,
{
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        self.iter().flat_map(|c| c.class_iter())
    }
}

impl<T: AsClassIter, const N: usize> AsClassIter for [T; N] {
    fn class_iter(&self) -> impl Iterator<Item = CowStr> {
        self.iter().flat_map(|c| c.class_iter())
    }
}

pub trait WithClasses {
    fn start_class_modifier(&mut self);
    fn add_class(&mut self, class_name: CowStr);
    fn remove_class(&mut self, class_name: CowStr);
    fn end_class_modifier(&mut self);
    // TODO something like the following, but I'm not yet sure how to support that efficiently (and without much binary bloat)
    // The modifiers possibly have to be applied then...
    // fn classes(&self) -> impl Iterator<CowStr>;
    // maybe also something like:
    // fn has_class(&self, class_name: &str) -> bool
    // Need to find a use-case for this first though (i.e. a modifier needs to read previously added classes)
}

#[derive(Debug)]
enum ClassModifier {
    Remove(CowStr),
    Add(CowStr),
    EndMarker(usize),
}

#[derive(Debug, Default)]
pub struct ClassAttributes {
    // TODO maybe this attribute is redundant and can be formed just from the class_modifiers attribute
    classes: VecMap<CowStr, ()>,
    class_modifiers: Vec<ClassModifier>,
    class_name: String,
    idx: usize,
    start_idx: usize,
    dirty: bool,
    /// a flag necessary, such that `start_class_modifier` doesn't always overwrite the last changes in `View::build`
    build_finished: bool,
}

impl ClassAttributes {
    pub fn apply_class_changes(&mut self, element: &web_sys::Element) {
        if self.dirty {
            self.dirty = false;
            self.classes.clear();
            for modifier in &self.class_modifiers {
                match modifier {
                    ClassModifier::Remove(class_name) => {
                        self.classes.remove(class_name);
                    }
                    ClassModifier::Add(class_name) => {
                        self.classes.insert(class_name.clone(), ());
                    }
                    ClassModifier::EndMarker(_) => (),
                }
            }
            // intersperse would be the right way to do this, but avoid extra dependencies just for this (and otherwise it's unstable in std)...
            self.class_name.clear();
            let last_idx = self.classes.len() - 1;
            for (idx, class) in self.classes.keys().enumerate() {
                self.class_name += class;
                if idx != last_idx {
                    self.class_name += " ";
                }
            }
            element.set_class_name(&self.class_name);
        }
        self.build_finished = true;
    }
}

impl WithClasses for ClassAttributes {
    fn start_class_modifier(&mut self) {
        if self.build_finished {
            if self.idx == 0 {
                self.start_idx = 0;
            } else {
                let ClassModifier::EndMarker(start_idx) = self.class_modifiers[self.idx - 1] else {
                    unreachable!("this should not happen, as either `start_class_modifier` is happens first, or follows an end_class_modifier")
                };
                self.idx = start_idx;
                self.start_idx = start_idx;
            }
        }
    }

    fn add_class(&mut self, class_name: CowStr) {
        match self.class_modifiers.get_mut(self.idx) {
            Some(ClassModifier::Add(class)) if class == &class_name => (), // class modifier hasn't changed
            Some(modifier) => {
                self.dirty = true;
                *modifier = ClassModifier::Add(class_name);
            }
            None => {
                self.dirty = true;
                self.class_modifiers.push(ClassModifier::Add(class_name));
            }
        }
        self.idx += 1;
    }

    fn remove_class(&mut self, class_name: CowStr) {
        // Same code as add_class but with remove...
        match self.class_modifiers.get_mut(self.idx) {
            Some(ClassModifier::Remove(class)) if class == &class_name => (), // class modifier hasn't changed
            Some(modifier) => {
                self.dirty = true;
                *modifier = ClassModifier::Remove(class_name);
            }
            None => {
                self.dirty = true;
                self.class_modifiers.push(ClassModifier::Remove(class_name));
            }
        }
        self.idx += 1;
    }

    fn end_class_modifier(&mut self) {
        match self.class_modifiers.get_mut(self.idx) {
            Some(ClassModifier::EndMarker(_)) if !self.dirty => (), // class modifier hasn't changed
            Some(modifier) => {
                self.dirty = true;
                *modifier = ClassModifier::EndMarker(self.start_idx);
            }
            None => {
                self.dirty = true;
                self.class_modifiers
                    .push(ClassModifier::EndMarker(self.start_idx));
            }
        }
        self.idx += 1;
        self.start_idx = self.idx;
    }
}

impl WithClasses for ElementProps {
    fn add_class(&mut self, class_name: CowStr) {
        self.class_attributes.add_class(class_name);
    }

    fn remove_class(&mut self, class_name: CowStr) {
        self.class_attributes.remove_class(class_name);
    }

    fn start_class_modifier(&mut self) {
        self.class_attributes.start_class_modifier();
    }

    fn end_class_modifier(&mut self) {
        self.class_attributes.end_class_modifier();
    }
}

impl<E: DomNode<Props: WithClasses>> WithClasses for Pod<E> {
    fn add_class(&mut self, class_name: CowStr) {
        self.props.add_class(class_name);
    }

    fn remove_class(&mut self, class_name: CowStr) {
        self.props.remove_class(class_name);
    }

    fn start_class_modifier(&mut self) {
        self.props.start_class_modifier();
    }

    fn end_class_modifier(&mut self) {
        self.props.end_class_modifier();
    }
}

impl<E: DomNode<Props: WithClasses>> WithClasses for PodMut<'_, E> {
    fn add_class(&mut self, class_name: CowStr) {
        self.props.add_class(class_name);
    }

    fn remove_class(&mut self, class_name: CowStr) {
        self.props.remove_class(class_name);
    }

    fn start_class_modifier(&mut self) {
        self.props.start_class_modifier();
    }

    fn end_class_modifier(&mut self) {
        self.props.end_class_modifier();
    }
}

pub trait ElementWithClasses: for<'a> ViewElement<Mut<'a>: WithClasses> + WithClasses {}

impl<T> ElementWithClasses for T
where
    T: ViewElement + WithClasses,
    for<'a> T::Mut<'a>: WithClasses,
{
}

#[derive(Clone, Debug)]
pub struct Class<E, C, T, A> {
    el: E,
    classes: C,
    phantom: PhantomData<fn() -> (T, A)>,
}

impl<E, C, T, A> Class<E, C, T, A> {
    pub fn new(el: E, classes: C) -> Self {
        Class {
            el,
            classes,
            phantom: PhantomData,
        }
    }
}

impl<E, C, T, A> View<T, A, ViewCtx> for Class<E, C, T, A>
where
    T: 'static,
    A: 'static,
    C: AsClassIter + 'static,
    E: View<T, A, ViewCtx, Element: ElementWithClasses>,
{
    type Element = E::Element;

    type ViewState = E::ViewState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let (mut e, s) = self.el.build(ctx);
        e.start_class_modifier();
        for class in self.classes.class_iter() {
            e.add_class(class);
        }
        e.end_class_modifier();
        (e, s)
    }

    fn rebuild<'e>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'e, Self::Element>,
    ) -> Mut<'e, Self::Element> {
        // This has to happen, before any children are rebuilt, otherwise this state machine breaks...
        // The actual modifiers also have to happen after the children are rebuilt, see `add_class` below.
        element.start_class_modifier();
        let mut element = self.el.rebuild(&prev.el, view_state, ctx, element);
        for class in self.classes.class_iter() {
            element.add_class(class);
        }
        element.end_class_modifier();
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
    ) -> MessageResult<A> {
        self.el.message(view_state, id_path, message, app_state)
    }
}
