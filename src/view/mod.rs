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

// mod async_list;
mod button;
mod progress_bar;
mod switch;
// mod text;
// mod layout_observer;
// mod list;
// mod scroll_view;
// mod use_state;
mod linear_layout;
mod list;
#[allow(clippy::module_inception)]
mod view;

pub use xilem_core::{Id, IdPath, VecSplice};

pub use button::button;
pub use linear_layout::{h_stack, v_stack, LinearLayout};
pub use list::{list, List};
pub use progress_bar::progress;
pub use switch::switch;
pub use view::{Adapt, AdaptState, Cx, Memoize, View, ViewMarker, ViewSequence};
