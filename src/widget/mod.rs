// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! Common widgets.

#[allow(clippy::module_inception)]
mod widget;
mod widget_mut;
mod widget_pod;
mod widget_ref;
mod widget_state;

#[cfg(test)]
mod tests;

mod align;
mod button;
mod checkbox;
mod flex;
mod image;
mod label;
mod portal;
mod scroll_bar;
mod sized_box;
mod spinner;
mod split;
mod textbox;

pub use align::Align;
pub use button::Button;
pub use checkbox::Checkbox;
pub use flex::{Axis, CrossAxisAlignment, Flex, FlexParams, MainAxisAlignment};
pub use label::{Label, LineBreaking};
pub use portal::Portal;
pub use scroll_bar::ScrollBar;
pub use sized_box::SizedBox;
pub use spinner::Spinner;
pub use split::Split;
pub use textbox::TextBox;
pub use widget::StoreInWidgetMut;
#[doc(hidden)]
pub use widget::{Widget, WidgetId};
//#[doc(hidden)]
//pub use widget_ext::WidgetExt;
//pub use widget_wrapper::WidgetWrapper;
pub use widget_mut::WidgetMut;
pub use widget_pod::WidgetPod;
pub use widget_ref::WidgetRef;
pub use widget_state::WidgetState;

pub use self::image::Image;

pub use sized_box::BackgroundBrush;

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

use crate::{Affine, Size};

// These are based on https://api.flutter.dev/flutter/painting/BoxFit-class.html
/// Strategies for inscribing a rectangle inside another rectangle.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum FillStrat {
    /// As large as posible without changing aspect ratio of image and all of image shown
    #[default]
    Contain,
    /// As large as posible with no dead space so that some of the image may be clipped
    Cover,
    /// Fill the widget with no dead space, aspect ratio of widget is used
    Fill,
    /// Fill the hight with the images aspect ratio, some of the image may be clipped
    FitHeight,
    /// Fill the width with the images aspect ratio, some of the image may be clipped
    FitWidth,
    /// Do not scale
    None,
    /// Scale down to fit but do not scale up
    ScaleDown,
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
        BoxConstraints, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
        RenderContext, Size, StatusChange, Widget, WidgetId,
    };
}
