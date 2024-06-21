// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

mod adapt;
pub use adapt::{adapt, Adapt, AdaptThunk};

mod map_state;
pub use map_state::{map_state, MapState};

mod map_action;
pub use map_action::{map_action, MapAction};

mod memoize;
pub use memoize::{memoize, Memoize};

/// Statically typed alternatives to the type-erased [`AnyView`](`crate::AnyView`).
pub mod one_of;