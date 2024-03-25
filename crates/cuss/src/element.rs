// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use crate::Symbol;

#[derive(Debug)]
pub struct Element {
    pub tag: Symbol,
    pub id: Option<Symbol>,
    pub class: HashSet<Symbol>,
}
