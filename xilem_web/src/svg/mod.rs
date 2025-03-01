// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Contains opinionated views such as [`kurbo`] shapes which can be used in an svg context

pub(crate) mod common_attrs;
pub(crate) mod kurbo_shape;

pub use common_attrs::{Fill, Stroke, fill, stroke};
pub use peniko;
pub use peniko::kurbo;
