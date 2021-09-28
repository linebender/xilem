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
//mod checkbox;
mod flex;
//mod click;
mod label;
//mod textbox;

#[allow(clippy::module_inception)]
mod widget;

#[doc(hidden)]
pub use widget::{Widget, WidgetId};

pub use button::Button;
//pub use checkbox::Checkbox;
//pub use click::Click;
pub use flex::{Axis, CrossAxisAlignment, Flex, FlexParams, MainAxisAlignment};
pub use label::{Label, LabelText, LineBreaking, RawLabel};
//pub use textbox::TextBox;

//#[doc(hidden)]
//pub use widget_ext::WidgetExt;
//pub use widget_wrapper::WidgetWrapper;

/// The types required to implement a `Widget`.
///
/// # Structs
/// [`BoxConstraints`](../../struct.BoxConstraints.html)\
/// [`Env`](../../struct.Env.html)\
/// [`EventCtx`](../../struct.EventCtx.html)\
/// [`LayoutCtx`](../../struct.LayoutCtx.html)\
/// [`LifeCycleCtx`](../../struct.LifeCycleCtx.html)\
/// [`PaintCtx`](../../struct.PaintCtx.html)\
/// [`Size`](../../struct.Size.html)\
/// [`UpdateCtx`](../../struct.UpdateCtx.html)\
/// [`WidgetId`](../../struct.WidgetId.html)\
///
/// # Enums
/// [`Event`](../../enum.Event.html)\
/// [`LifeCycle`](../../enum.LifeCycle.html)\
///
/// # Traits
/// [`RenderContext`](../../trait.RenderContext.html)\
/// [`Widget`](../../trait.Widget.html)
// NOTE: \ at the end works as a line break, but skip on last line!
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AsWidgetPod, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
        PaintCtx, RenderContext, Size, UpdateCtx, Widget, WidgetId,
    };
}
