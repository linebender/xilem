// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Datatypes representing selectors in a stylesheet.

use crate::{Element, Symbol};

#[derive(Debug)]
pub enum TypeSelector {
    Wild,
    Ident(Symbol),
}

#[derive(Debug)]
pub struct CompoundSelector {
    pub type_selector: Option<TypeSelector>,
    // Grammar has vec of subclass selectors, but we'll be more structured.
    pub id_selector: Option<Symbol>,
    pub class_selectors: Vec<Symbol>,
}

#[derive(PartialEq, Debug)]
pub enum Combinator {
    Descendant,
    Child,
}

#[derive(Debug)]
pub struct ComplexSelector {
    pub first: CompoundSelector,
    pub tail: Vec<(Combinator, CompoundSelector)>,
}

impl TypeSelector {
    pub fn matches(&self, el: &Element) -> bool {
        match self {
            TypeSelector::Wild => true,
            TypeSelector::Ident(id) => *id == el.tag,
        }
    }
}

impl CompoundSelector {
    pub fn matches(&self, el: &Element) -> bool {
        if let Some(type_sel) = &self.type_selector {
            if !type_sel.matches(el) {
                return false;
            }
        }
        if self.id_selector.is_some() && self.id_selector != el.id {
            return false;
        }
        // TODO: match classes
        true
    }
}
