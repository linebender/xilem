// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code, unreachable_pub)]

use core::marker::PhantomData;

use crate::{
    AppendVec, ElementSplice, MessageContext, MessageResult, SuperElement, ViewElement, ViewMarker,
    ViewPathTracker,
};

#[expect(unreachable_pub)]
pub trait Marker {}

pub enum Base {}

impl Marker for Base {}

pub struct Converted<Element> {
    phantom: PhantomData<Element>,
    base: Base,
}

impl<Element> Marker for Converted<Element> {}

pub enum Count {}

impl Count {
    const ZERO: u8 = 0;
    const ONE: u8 = 1;
    const MANY: u8 = 2;
    const UNKNOWN: u8 = 255;
}

// What properties would we like:
// 1) If we have Views<Element>, we want it to also implement `Views<DynElement>`
//    For sequences (e.g. tuples, etc.), it's not viable to make that an explicit `.as()`
// 2) Corrolery: If we have `Views<!>`, it should be includable everywhere
pub trait Views<M: Marker, State, Action, Context>: 'static
where
    Context: ViewPathTracker,
{
    type Element: ViewElement;
    const COUNT: u8;
}

impl<V, State, Action, Context, Element> Views<Converted<Element>, State, Action, Context> for V
where
    Context: ViewPathTracker,
    Element: SuperElement<V::Element, Context>,
    V: Views<Base, State, Action, Context> + ViewMarker,
{
    type Element = Element;

    const COUNT: u8 = V::COUNT;
}

impl<V, State, Action, Context, Element> Views<Converted<Element>, State, Action, Context> for (V,)
where
    Context: ViewPathTracker,
    Element: ViewElement,
    V: Views<Converted<Element>, State, Action, Context>,
{
    type Element = Element;

    const COUNT: u8 = V::COUNT;
}

impl<V0, V1, State, Action, Context, Element> Views<Converted<Element>, State, Action, Context>
    for (V0, V1)
where
    Context: ViewPathTracker,
    Element: ViewElement,
    V0: Views<Converted<Element>, State, Action, Context>,
    V1: Views<Converted<Element>, State, Action, Context>,
{
    type Element = Element;

    const COUNT: u8 = combine_counts([V0::COUNT, V1::COUNT]);
}

const fn combine_counts<const N: usize>(vals: [u8; N]) -> u8 {
    let mut idx = 0;
    let mut current_count = Count::ZERO;
    while idx < N {
        idx += 1;
        match vals[idx] {
            // Zero does nothing
            Count::ZERO => {}
            Count::ONE if current_count == Count::ZERO => {
                current_count = Count::ONE;
            }
            Count::ONE if current_count == Count::ONE => {
                current_count = Count::MANY;
            }
            Count::ONE => {}
            Count::MANY => {
                current_count = Count::MANY;
            }
            Count::UNKNOWN if current_count != Count::MANY => {
                current_count = Count::UNKNOWN;
            }
            _ => panic!("How to report this properly"),
        }
    }
    current_count
}
