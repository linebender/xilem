// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Datatypes representing rules and values in a stylesheet.

use crate::{selector::ComplexSelector, Symbol};

#[derive(Debug)]
pub struct Stylesheet(pub Vec<Rule>);

#[derive(Debug)]
pub struct Rule {
    pub selectors: Vec<ComplexSelector>,
    pub decls: Vec<Declaration>,
}

#[derive(Debug)]
pub struct Rules(Vec<Rule>);

/// A parsed value.
#[derive(Debug, Clone)]
pub enum Value {
    Symbol(Symbol),
    Number(f64),
    Percent(f64),
    Dimension(f64, Symbol),
    String(String),
    Function(Symbol, Vec<Value>),
    Color(u32),
}

#[derive(Debug)]
pub struct Declaration {
    pub name: Symbol,
    // TODO: may need two-level hierarchy for values, for comma separated lists
    // Alternatively, comma variant in Value enum?
    pub values: Vec<Value>,
}
