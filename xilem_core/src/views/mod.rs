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

mod one_of;
pub use one_of::{
    OneOf2, OneOf2Ctx, OneOf3, OneOf3Ctx, OneOf4, OneOf4Ctx, OneOf5, OneOf5Ctx, OneOf6, OneOf6Ctx,
    OneOf7, OneOf7Ctx, OneOf8, OneOf8Ctx, OneOf9, OneOf9Ctx,
};

mod orphan;
pub use orphan::{AsOrphanView, OrphanView};
