// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
/// A stable identifier for an element.
pub struct Id(NonZeroU64);

impl WidgetId {
    /// Allocate a new, unique `Id`.
    pub fn next() -> WidgetId {
        use std::sync::atomic::{AtomicU64, Ordering};
        static WIDGET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        WidgetId(
            WIDGET_ID_COUNTER
                .fetch_add(1, Ordering::Relaxed)
                .try_into()
                .unwrap(),
        )
    }

    pub fn to_raw(self) -> u64 {
        self.0.into()
    }

    pub fn to_nonzero_raw(self) -> NonZeroU64 {
        self.0
    }
}
