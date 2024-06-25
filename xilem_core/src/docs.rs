// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Fake implementations of Xilem traits for use within documentation examples and tests.

use crate::ViewPathTracker;

/// A type used for documentation
pub enum Fake {}

impl ViewPathTracker for Fake {
    fn push_id(&mut self, _: crate::ViewId) {
        match *self {}
    }
    fn pop_id(&mut self) {
        match *self {}
    }

    fn view_path(&mut self) -> &[crate::ViewId] {
        match *self {}
    }
}
