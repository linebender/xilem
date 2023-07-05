// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! State machine implementation.

use std::cmp::Ordering;

use crate::{
    selector::{Combinator, ComplexSelector, CompoundSelector, TypeSelector},
    Symbol,
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Cursor {
    rule_ix: usize,
    sel_ix: usize,
    sel_state: SelState,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct NfaState(Vec<Cursor>);

#[derive(Debug, PartialEq, Eq, Hash)]
enum SelState {
    Init,
    Tag,
    Class(usize),
    Final,
}

impl Cursor {
    fn rule_sel(&self) -> (usize, usize) {
        (self.rule_ix, self.sel_ix)
    }

    fn rule_sel_next(&self) -> (usize, usize) {
        (self.rule_ix, self.sel_ix + 1)
    }
}

impl Cursor {
    fn get_sel<'a>(&self, sels: &'a [ComplexSelector]) -> &'a CompoundSelector {
        let comp = &sels[self.rule_ix];
        if self.sel_ix == 0 {
            &comp.first
        } else {
            &comp.tail[self.sel_ix - 1].1
        }
    }

    fn step_id(&self, sels: &[ComplexSelector], id: Option<Symbol>) -> Option<Cursor> {
        let sel = self.get_sel(sels);
        if sel.id_selector.is_some() && sel.id_selector != id {
            return None;
        }
        Some(Cursor {
            rule_ix: self.rule_ix,
            sel_ix: self.sel_ix,
            sel_state: SelState::Tag,
        })
    }

    fn step_tag(&self, sels: &[ComplexSelector], tag: Symbol) -> Option<Cursor> {
        let sel = self.get_sel(sels);
        if let Some(TypeSelector::Ident(sel_tag)) = &sel.type_selector {
            if *sel_tag != tag {
                return None;
            }
        }
        Some(Cursor {
            rule_ix: self.rule_ix,
            sel_ix: self.sel_ix,
            sel_state: SelState::Class(0),
        })
    }

    fn step_class(&self, sels: &[ComplexSelector], class: Symbol) -> Option<Cursor> {
        if let SelState::Class(mut class_ix) = self.sel_state {
            let sel = self.get_sel(sels);
            if let Some(sel_class) = sel.class_selectors.get(class_ix) {
                match class.cmp(sel_class) {
                    Ordering::Less => (),
                    Ordering::Equal => class_ix += 1,
                    Ordering::Greater => return None,
                }
            }
            Some(Cursor {
                rule_ix: self.rule_ix,
                sel_ix: self.sel_ix,
                sel_state: SelState::Class(class_ix),
            })
        } else {
            None
        }
    }

    fn end_class(&self, sels: &[ComplexSelector]) -> Option<Cursor> {
        if let SelState::Class(class_ix) = self.sel_state {
            let sel = self.get_sel(sels);
            if class_ix == sel.class_selectors.len() {
                return Some(Cursor {
                    rule_ix: self.rule_ix,
                    sel_ix: self.sel_ix,
                    sel_state: SelState::Final,
                });
            }
        }
        None
    }

    pub(crate) fn accepting_rule(&self, sels: &[ComplexSelector]) -> Option<usize> {
        if self.sel_ix == sels[self.rule_ix].tail.len() {
            Some(self.rule_ix)
        } else {
            None
        }
    }
}

impl NfaState {
    pub fn initial(sels: &[ComplexSelector]) -> NfaState {
        NfaState(
            (0..sels.len())
                .map(|rule_ix| Cursor {
                    rule_ix,
                    sel_ix: 0,
                    sel_state: SelState::Init,
                })
                .collect(),
        )
    }

    pub fn step_id(&self, sels: &[ComplexSelector], id: Option<Symbol>) -> NfaState {
        NfaState(self.0.iter().flat_map(|c| c.step_id(sels, id)).collect())
    }

    pub fn step_tag(&self, sels: &[ComplexSelector], tag: Symbol) -> NfaState {
        NfaState(self.0.iter().flat_map(|c| c.step_tag(sels, tag)).collect())
    }

    pub fn step_class(&self, sels: &[ComplexSelector], class: Symbol) -> NfaState {
        NfaState(
            self.0
                .iter()
                .flat_map(|c| c.step_class(sels, class))
                .collect(),
        )
    }

    pub fn end_class(&self, sels: &[ComplexSelector]) -> NfaState {
        NfaState(self.0.iter().flat_map(|c| c.end_class(sels)).collect())
    }

    /// Merge a base state and a tip state.
    ///
    /// The tip state is assumed to only be in `SelState::Final`. This method applies
    /// NFA logic, keeping the base state if the combinator is `Descendant`.
    pub fn merge(&self, tip: &NfaState, sels: &[ComplexSelector]) -> NfaState {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.0.len() || j < tip.0.len() {
            let (rule_ix, sel_ix, is_tip) = if j == tip.0.len() {
                let rs = self.0[i].rule_sel();
                i += 1;
                (rs.0, rs.1, false)
            } else if i == self.0.len() {
                let rs = tip.0[j].rule_sel_next();
                j += 1;
                (rs.0, rs.1, true)
            } else {
                let rs0 = self.0[i].rule_sel();
                let rs1 = tip.0[j].rule_sel_next();
                match rs0.cmp(&rs1) {
                    Ordering::Less => {
                        i += 1;
                        (rs0.0, rs0.1, false)
                    }
                    Ordering::Equal => {
                        i += 1;
                        j += 1;
                        (rs1.0, rs1.1, true)
                    }
                    Ordering::Greater => {
                        j += 1;
                        (rs1.0, rs1.1, true)
                    }
                }
            };
            if sel_ix < sels[rule_ix].tail.len() + 1
                && (is_tip
                    || sel_ix == 0
                    || sels[rule_ix].tail[sel_ix - 1].0 == Combinator::Descendant)
            {
                result.push(Cursor {
                    rule_ix,
                    sel_ix,
                    sel_state: SelState::Init,
                })
            }
        }
        NfaState(result)
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.0
    }
}
