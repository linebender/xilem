// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

mod impl_array;
mod impl_option;
mod impl_tuples;
mod impl_vec;
mod without_elements;

pub(crate) use self::without_elements::NoElements;
pub use self::without_elements::{WithoutElements, without_elements};
