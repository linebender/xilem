// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Resolve a stylesheet.

use std::collections::HashMap;

use crate::{
    rules::{Declaration, Stylesheet, Value},
    MatchState, Matcher, Symbol,
};

pub struct Resolver {
    matcher: Matcher,
    // TODO: probably will also include specificity
    decl_ixs: Vec<usize>,
    decls: Vec<Vec<Declaration>>,

    resolved_props: Vec<Properties>,
    // Map parent resolve state and step_class_end result to child state.
    transitions: HashMap<(ResolveState, MatchState), ResolveState>,
}

/// Resolved properties.
#[derive(Debug, Clone, Default)]
pub struct Properties {
    /// The merged state.
    next_state: MatchState,
    props: HashMap<Symbol, Vec<Value>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct ResolveState(usize);

pub struct MatchTip {
    parent: ResolveState,
    base: MatchState,
    tip: MatchState,
}

impl Resolver {
    pub fn new(stylesheet: Stylesheet) -> Self {
        let mut sels = vec![];
        let mut decl_ixs = vec![];
        let mut decls = vec![];
        for (i, rule) in stylesheet.0.into_iter().enumerate() {
            decl_ixs.resize(decl_ixs.len() + rule.selectors.len(), i);
            sels.extend(rule.selectors);
            decls.push(rule.decls);
        }
        let matcher = Matcher::new(sels);
        let resolved_props = vec![Default::default()];
        let transitions = Default::default();
        Resolver {
            matcher,
            decl_ixs,
            decls,
            resolved_props,
            transitions,
        }
    }

    pub fn step_id(&mut self, parent: ResolveState, id: Option<Symbol>) -> MatchTip {
        let base = self.resolved_props[parent.0].next_state;
        let tip = self.matcher.step_id(base, id);
        MatchTip { parent, base, tip }
    }

    pub fn step_tag(&mut self, state: MatchTip, tag: Symbol) -> MatchTip {
        let tip = self.matcher.step_tag(state.tip, tag);
        MatchTip {
            parent: state.parent,
            base: state.base,
            tip,
        }
    }

    pub fn step_class(&mut self, state: MatchTip, class: Symbol) -> MatchTip {
        let tip = self.matcher.step_class(state.tip, class);
        MatchTip {
            parent: state.parent,
            base: state.base,
            tip,
        }
    }

    // Note: can probably get rid of this, subsume it in resolve.
    pub fn step_class_end(&mut self, state: MatchTip) -> MatchTip {
        let tip = self.matcher.step_class_end(state.tip);
        MatchTip {
            parent: state.parent,
            base: state.base,
            tip,
        }
    }

    /// Apply selector matches.
    pub fn resolve(&mut self, state: MatchTip) -> ResolveState {
        *self
            .transitions
            .entry((state.parent, state.tip))
            .or_insert_with(|| {
                let next_state = self.matcher.merge(state.base, state.tip);
                let nfa_state = self.matcher.tip_state(state.tip);
                let mut props = self.resolved_props[state.parent.0].make_child(next_state);
                for cursor in nfa_state.cursors() {
                    if let Some(rule_ix) = self.matcher.accepting_rule(cursor) {
                        let decl = &self.decls[self.decl_ixs[rule_ix]];
                        props.apply_decls(decl);
                    }
                }
                let new_resolved_state = ResolveState(self.resolved_props.len());
                self.resolved_props.push(props);
                new_resolved_state
            })
    }

    /// Get properties for a resolve state.
    ///
    /// Panics when the state is invalid (this can only happen when a state
    /// from a different resolver is used).
    pub fn props(&self, state: ResolveState) -> &Properties {
        &self.resolved_props[state.0]
    }
}

impl Properties {
    // This is currently primitive, should apply "inherit" and other
    // cascading rules.
    fn apply_decls(&mut self, decls: &[Declaration]) {
        for decl in decls {
            self.props.insert(decl.name, decl.values.clone());
        }
    }

    // TODO: shouldn't clone all values, only inherited
    fn make_child(&self, next_state: MatchState) -> Self {
        Properties {
            next_state,
            props: self.props.clone(),
        }
    }
}
