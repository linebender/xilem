// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs, reason = "WIP")]

pub(crate) mod action;
pub(crate) mod box_constraints;
pub mod contexts;
pub mod event;
pub mod text;

pub(crate) mod widget;
mod widget_arena;
mod widget_mut;
mod widget_pod;
mod widget_ref;
mod widget_state;

pub use action::Action;
pub use box_constraints::BoxConstraints;
pub use contexts::AccessCtx;
pub use contexts::ComposeCtx;
pub use contexts::EventCtx;
pub use contexts::IsContext;
pub use contexts::LayoutCtx;
pub use contexts::MutateCtx;
pub use contexts::PaintCtx;
pub use contexts::QueryCtx;
pub use contexts::RawWrapper;
pub use contexts::RawWrapperMut;
pub use contexts::RegisterCtx;
pub use contexts::UpdateCtx;
pub use event::AccessEvent;
pub use event::PointerButton;
pub use event::PointerEvent;
pub use event::PointerState;
pub use event::TextEvent;
pub use event::Update;
pub use event::WindowEvent;
pub use event::WindowTheme;
pub use text::render_text;
pub use text::ArcStr;
pub use text::BrushIndex;
pub use text::StyleProperty;
pub use text::StyleSet;

pub use object_fit::ObjectFit;
pub use widget::AllowRawMut;
pub use widget::FromDynWidget;
pub use widget::Widget;
pub use widget::WidgetId;
pub use widget_mut::WidgetMut;
pub use widget_pod::WidgetPod;
pub use widget_ref::WidgetRef;

pub(crate) use text::default_styles;
pub(crate) use widget_arena::WidgetArena;
pub(crate) use widget_pod::CreateWidget;
pub(crate) use widget_state::WidgetState;

mod object_fit {
    use crate::Affine;
    use crate::Size;

    // These are based on https://developer.mozilla.org/en-US/docs/Web/CSS/object-fit
    /// Strategies for inscribing a rectangle inside another rectangle.
    #[derive(Clone, Copy, Default, PartialEq)]
    pub enum ObjectFit {
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

    impl ObjectFit {
        /// Calculate an origin and scale for an image with a given `ObjectFit`.
        ///
        /// This takes some properties of a widget and an object fit and returns an affine matrix
        /// used to position and scale the image in the widget.
        pub fn affine_to_fill(self, parent: Size, fit_box: Size) -> Affine {
            let raw_scalex = parent.width / fit_box.width;
            let raw_scaley = parent.height / fit_box.height;

            let (scalex, scaley) = match self {
                Self::Contain => {
                    let scale = raw_scalex.min(raw_scaley);
                    (scale, scale)
                }
                Self::Cover => {
                    let scale = raw_scalex.max(raw_scaley);
                    (scale, scale)
                }
                Self::Fill => (raw_scalex, raw_scaley),
                Self::FitHeight => (raw_scaley, raw_scaley),
                Self::FitWidth => (raw_scalex, raw_scalex),
                Self::ScaleDown => {
                    let scale = raw_scalex.min(raw_scaley).min(1.0);
                    (scale, scale)
                }
                Self::None => (1.0, 1.0),
            };

            let origin_x = (parent.width - (fit_box.width * scalex)) / 2.0;
            let origin_y = (parent.height - (fit_box.height * scaley)) / 2.0;

            Affine::new([scalex, 0., 0., scaley, origin_x, origin_y])
        }
    }
}
