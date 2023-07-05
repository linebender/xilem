// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Nondeterministic finite automaton matcher for a set of selectors.

use std::collections::HashMap;

use indexmap::IndexSet;

use crate::{
    selector::ComplexSelector,
    statemachine::{Cursor, NfaState},
    Symbol,
};

pub struct Matcher {
    sels: Vec<ComplexSelector>,
    states: IndexSet<NfaState>,
    id_xn: HashMap<(usize, Option<Symbol>), usize>,
    tag_xn: HashMap<(usize, Symbol), usize>,
    class_xn: HashMap<(usize, Symbol), usize>,
    class_end_xn: HashMap<usize, usize>,
    merge_xn: HashMap<(MatchState, MatchState), usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct MatchState(usize);

impl Matcher {
    pub fn new(sels: Vec<ComplexSelector>) -> Matcher {
        let init_state = NfaState::initial(&sels);
        let mut states = IndexSet::new();
        states.insert(init_state);
        Matcher {
            sels,
            states,
            id_xn: Default::default(),
            tag_xn: Default::default(),
            class_xn: Default::default(),
            class_end_xn: Default::default(),
            merge_xn: Default::default(),
        }
    }

    /// The state should be a default or merged state.
    pub fn step_id(&mut self, state: MatchState, id: Option<Symbol>) -> MatchState {
        let base_state = state.0;
        let tip_state = *self.id_xn.entry((base_state, id)).or_insert_with(|| {
            let next_state = self.states[base_state].step_id(&self.sels, id);
            self.states.insert_full(next_state).0
        });
        MatchState(tip_state)
    }

    /// The state should be the result of a `step_id` call.
    pub fn step_tag(&mut self, state: MatchState, tag: Symbol) -> MatchState {
        let tip_state = *self.tag_xn.entry((state.0, tag)).or_insert_with(|| {
            let next_state = self.states[state.0].step_tag(&self.sels, tag);
            self.states.insert_full(next_state).0
        });
        MatchState(tip_state)
    }

    /// The state should be the result of a `step_tag` or `step_class` call.
    pub fn step_class(&mut self, state: MatchState, class: Symbol) -> MatchState {
        let tip_state = *self.class_xn.entry((state.0, class)).or_insert_with(|| {
            let next_state = self.states[state.0].step_class(&self.sels, class);
            self.states.insert_full(next_state).0
        });
        MatchState(tip_state)
    }

    /// The state should be the result of a `step_tag` or `step_class` call.
    ///
    /// The resulting state is suitable for detecting matches.
    pub fn step_class_end(&mut self, state: MatchState) -> MatchState {
        let tip_state = *self.class_end_xn.entry(state.0).or_insert_with(|| {
            let next_state = self.states[state.0].end_class(&self.sels);
            self.states.insert_full(next_state).0
        });
        MatchState(tip_state)
    }

    /// The base_state should be a default or merged state, the `state` arg
    /// should be the result of a `step_class_end` call.
    pub fn merge(&mut self, base_state: MatchState, state: MatchState) -> MatchState {
        MatchState(
            *self.merge_xn.entry((base_state, state)).or_insert_with(|| {
                let next_state = self.states[base_state.0].merge(&self.states[state.0], &self.sels);
                self.states.insert_full(next_state).0
            }),
        )
    }

    pub fn dump_state(&self, state: MatchState) {
        println!("{:#?}", self.states[state.0]);
    }

    pub(crate) fn tip_state(&self, tip: MatchState) -> &NfaState {
        &self.states[tip.0]
    }

    pub(crate) fn accepting_rule(&self, cursor: &Cursor) -> Option<usize> {
        cursor.accepting_rule(&self.sels)
    }
}
