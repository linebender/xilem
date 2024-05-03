// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Generic implementation of Xilem view traits.
//!
//! This crate has a few basic types needed to support views, and also
//! a set of macros used to instantiate the main view traits. The client
//! will need to supply a bound on elements, a "pod" type which
//! supports dynamic dispatching and marking of change flags, and a
//! context.
//!
//! All this is still experimental. This crate is where more of the core
//! Xilem architecture will land (some of which was implemented in the
//! original prototype but not yet ported): adapt, memoize, use_state,
//! and possibly some async logic. Likely most of env will also land
//! here, but that also requires coordination with the context.

mod any_view;
mod id;
mod message;
mod sequence;
mod vec_splice;
mod view;

pub use id::{Id, IdPath};
pub use message::{AsyncWake, MessageResult};
pub use vec_splice::VecSplice;
