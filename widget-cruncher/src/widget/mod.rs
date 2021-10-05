// Copyright 2018 The Druid Authors.
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

//! Common widgets.

mod button;
mod flex;
mod label;
mod sized_box;
//mod checkbox;
//mod click;
//mod textbox;

#[allow(clippy::module_inception)]
mod widget;
mod widget_pod;
mod widget_state;

#[doc(hidden)]
pub use widget::{Widget, WidgetId};

//pub use checkbox::Checkbox;
//pub use click::Click;
//pub use textbox::TextBox;
pub use button::Button;
pub use flex::{Axis, CrossAxisAlignment, Flex, FlexParams, MainAxisAlignment};
pub use label::{Label, LineBreaking};
pub use sized_box::SizedBox;

//#[doc(hidden)]
//pub use widget_ext::WidgetExt;
//pub use widget_wrapper::WidgetWrapper;

pub use widget_pod::{AsWidgetPod, WidgetPod};
pub use widget_state::WidgetState;

/// Methods by which a widget can attempt to change focus state.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FocusChange {
    /// The focused widget is giving up focus.
    Resign,
    /// A specific widget wants focus
    Focus(WidgetId),
    /// Focus should pass to the next focusable widget
    Next,
    /// Focus should pass to the previous focusable widget
    Previous,
}

/// The possible cursor states for a widget.
#[derive(Clone, Debug)]
pub(crate) enum CursorChange {
    /// No cursor has been set.
    Default,
    /// Someone set a cursor, but if a child widget also set their cursor then we'll use theirs
    /// instead of ours.
    Set(druid_shell::Cursor),
    /// Someone set a cursor, and we'll use it regardless of what the children say.
    Override(druid_shell::Cursor),
}

// TODO
impl CursorChange {
    pub fn cursor(&self) -> Option<druid_shell::Cursor> {
        match self {
            CursorChange::Set(c) | CursorChange::Override(c) => Some(c.clone()),
            CursorChange::Default => None,
        }
    }
}

// TODO - remove
pub mod prelude {
    pub use crate::event::StatusChange;
    #[doc(hidden)]
    pub use crate::{
        AsWidgetPod, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
        PaintCtx, RenderContext, Size, Widget, WidgetId,
    };
}
