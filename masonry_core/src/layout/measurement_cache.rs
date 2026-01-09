// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::kurbo::Axis;
use crate::layout::LenReq;

/// All the inputs that change [`measure`] output.
///
/// Notably this doesn't include properties, because the widget is
/// responsible for requesting layout when a relevant property changes.
///
/// [`measure`]: crate::core::Widget::measure
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct MeasurementInputs {
    axis: Axis,
    len_req: LenReq,
    cross_length: Option<f64>,
}

impl MeasurementInputs {
    /// Creates a new [`MeasurementInputs`] with the provided data.
    pub(crate) const fn new(axis: Axis, len_req: LenReq, cross_length: Option<f64>) -> Self {
        Self {
            axis,
            len_req,
            cross_length,
        }
    }
}

/// At the time of choosing this capacity,
/// 10 * 48 bytes = 480 bytes for the whole buffer.
const CAPACITY: usize = 10;

/// Contains a mapping of [`MeasurementInputs`] to results.
///
/// Implemented as a linear search LRU cache over [`Vec`],
/// because we expect the dataset to be tiny.
///
/// We don't expect any `NaN`s to be in [`MeasurementInputs`],
/// but even if there are that is fine, because we clear the cache regularly.
#[derive(Clone, Debug)]
pub(crate) struct MeasurementCache {
    entries: Vec<(MeasurementInputs, f64)>,
}

impl MeasurementCache {
    /// Creates a new [`MeasurementCache`].
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::with_capacity(CAPACITY),
        }
    }

    /// Inserts the `result` for the given `inputs` into the cache.
    pub(crate) fn insert(&mut self, inputs: MeasurementInputs, result: f64) {
        if let Some(index) = self.entries.iter().position(|e| e.0 == inputs) {
            if index > 0 {
                // Keep recently referenced entries in front
                self.entries[0..=index].rotate_right(1);
            }
            // We already have this entry, but make sure its value is updated
            self.entries[0].1 = result;
            return;
        }
        // We never grow beyond our capacity
        if self.entries.len() == CAPACITY {
            // The last entry is the least recently used
            self.entries.pop();
        }
        // New entries go to the front
        self.entries.insert(0, (inputs, result));
    }

    /// Gets the cached result for the given `inputs`.
    pub(crate) fn get(&mut self, inputs: &MeasurementInputs) -> Option<f64> {
        let index = self.entries.iter().position(|e| &e.0 == inputs)?;
        if index > 0 {
            // Keep recently referenced entries in front
            self.entries[0..=index].rotate_right(1);
        }
        Some(self.entries[0].1)
    }

    /// Clears the cache.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}
