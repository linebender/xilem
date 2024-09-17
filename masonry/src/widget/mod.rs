// Copyright 2018 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Common widgets.

#[allow(clippy::module_inception)]
pub(crate) mod widget;
mod widget_mut;
mod widget_pod;
mod widget_ref;
mod widget_state;

#[cfg(test)]
mod tests;

mod align;
mod board;
mod button;
mod checkbox;
mod flex;
mod grid;
mod image;
mod label;
mod portal;
mod progress_bar;
mod prose;
mod root_widget;
mod scroll_bar;
mod sized_box;
mod spinner;
mod split;
mod textbox;
mod variable_label;
mod widget_arena;

pub use self::image::Image;
pub use align::Align;
pub use board::{Board, BoardParams, ConcreteShape, KurboShape};
pub use button::Button;
pub use checkbox::Checkbox;
pub use flex::{Axis, CrossAxisAlignment, Flex, FlexParams, MainAxisAlignment};
pub use grid::{Grid, GridParams};
pub use label::{Label, LineBreaking};
pub use portal::Portal;
pub use progress_bar::ProgressBar;
pub use prose::Prose;
pub use root_widget::RootWidget;
pub use scroll_bar::ScrollBar;
pub use sized_box::SizedBox;
pub use spinner::Spinner;
pub use split::Split;
pub use textbox::Textbox;
pub use variable_label::VariableLabel;
pub use widget_mut::WidgetMut;
pub use widget_pod::WidgetPod;
pub use widget_ref::WidgetRef;
pub use widget_state::WidgetState;

pub(crate) use widget_arena::WidgetArena;

use crate::{Affine, Size};

// These are based on https://api.flutter.dev/flutter/painting/BoxFit-class.html
/// Strategies for inscribing a rectangle inside another rectangle.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum FillStrat {
    /// As large as possible without changing aspect ratio of image and all of image shown
    #[default]
    Contain,
    /// As large as possible with no dead space so that some of the image may be clipped
    Cover,
    /// Fill the widget with no dead space, aspect ratio of widget is used
    Fill,
    /// Fill the height with the images aspect ratio, some of the image may be clipped
    FitHeight,
    /// Fill the width with the images aspect ratio, some of the image may be clipped
    FitWidth,
    /// Do not scale
    None,
    /// Scale down to fit but do not scale up
    ScaleDown,
}

// TODO - Need to write tests for this, in a way that's relatively easy to visualize.

impl FillStrat {
    /// Calculate an origin and scale for an image with a given `FillStrat`.
    ///
    /// This takes some properties of a widget and a fill strategy and returns an affine matrix
    /// used to position and scale the image in the widget.
    pub fn affine_to_fill(self, parent: Size, fit_box: Size) -> Affine {
        let raw_scalex = parent.width / fit_box.width;
        let raw_scaley = parent.height / fit_box.height;

        let (scalex, scaley) = match self {
            FillStrat::Contain => {
                let scale = raw_scalex.min(raw_scaley);
                (scale, scale)
            }
            FillStrat::Cover => {
                let scale = raw_scalex.max(raw_scaley);
                (scale, scale)
            }
            FillStrat::Fill => (raw_scalex, raw_scaley),
            FillStrat::FitHeight => (raw_scaley, raw_scaley),
            FillStrat::FitWidth => (raw_scalex, raw_scalex),
            FillStrat::ScaleDown => {
                let scale = raw_scalex.min(raw_scaley).min(1.0);
                (scale, scale)
            }
            FillStrat::None => (1.0, 1.0),
        };

        let origin_x = (parent.width - (fit_box.width * scalex)) / 2.0;
        let origin_y = (parent.height - (fit_box.height * scaley)) / 2.0;

        Affine::new([scalex, 0., 0., scaley, origin_x, origin_y])
    }
}

// TODO - remove prelude
#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, PointerEvent, Size,
        StatusChange, TextEvent, Widget, WidgetId,
    };
}
