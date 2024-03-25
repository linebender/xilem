// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An interned string pool.

use indexmap::IndexSet;

/// An interned symbol.
/// 
/// Note that the representation is a string for debugging purposes, but
/// intent is to change that to a simple usize, for better performance
/// (and to avoid leaking memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(&'static str);

#[derive(Default)]
pub struct SymbolPool(IndexSet<&'static str>);

macro_rules! known_symbols {
    ($( $name: ident : $value: literal ),*) => {
        impl Symbol {
            $(
            pub const $name: Symbol = Symbol($value);
            )*
        }
        impl SymbolPool {
            pub fn new() -> Self {
                let mut result = IndexSet::default();
                $(
                    result.insert($value);
                )*
                SymbolPool(result)
            }
        }
    }
}

known_symbols!(
    A: "a",
    BODY: "body",
    DIV: "div",
    H1: "h1",
    H2: "h2",
    H3: "h3",
    H4: "h4",
    H5: "h5",
    P: "p",
    LI: "li",
    DISPLAY: "display",
    BLOCK: "block"
);

impl SymbolPool {
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(sym) = self.0.get(s) {
            return Symbol(sym);
        }
        let result = Box::leak(Box::from(s));
        self.0.insert(result);
        Symbol(result)
    }
}
