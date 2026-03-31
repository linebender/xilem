// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use masonry::kurbo::{Axis, Cap};
use masonry::layout::Length;
use masonry::util::debug_panic;
use masonry::widgets::{self, DashFit, Placement};
use smallvec::SmallVec;

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
use crate::view::Spinner;
use crate::{Pod, ViewCtx, WidgetView};

/// Creates a new [`Divider`] parallel with the given `axis`.
///
/// Optionally the divider can also display `content`.
pub fn divider<State: 'static, Action, V: WidgetView<State, Action>>(
    axis: Axis,
    content: impl Into<Option<V>>,
) -> Divider<State, Action, V> {
    Divider {
        axis,
        thickness: None,
        dash_fit: DashFit::default(),
        dash_pattern: SmallVec::default(),
        start_cap: Cap::Butt,
        end_cap: Cap::Butt,
        placement: Placement::default(),
        pad: Length::const_px(5.),
        content: content.into(),

        phantom: PhantomData,
    }
}

/// Creates a new horizontal [`Divider`] with no content.
pub fn divider_h<State: 'static, Action>() -> Divider<State, Action> {
    divider(Axis::Horizontal, None)
}

/// Creates a new vertical [`Divider`] with no content.
pub fn divider_v<State: 'static, Action>() -> Divider<State, Action> {
    divider(Axis::Vertical, None)
}

/// A line to divide your content.
///
/// By default it is a thin solid line. A dash pattern can be configured with [`dash_pattern`].
///
/// [`dash_pattern`]: Self::dash_pattern
// Using Spinner as a dummy default so Divider can be constructed without a manual content type.
pub struct Divider<State, Action, V = Spinner> {
    axis: Axis,
    /// No set thickness means hairline - 1 device pixel.
    thickness: Option<Length>,
    dash_fit: DashFit,
    dash_pattern: SmallVec<[Length; 2]>,
    start_cap: Cap,
    end_cap: Cap,
    placement: Placement,
    pad: Length,
    content: Option<V>,

    phantom: PhantomData<fn(State) -> Action>,
}

impl<State: 'static, Action, V: WidgetView<State, Action>> Divider<State, Action, V> {
    /// Returns `self` with the given line `thickness`.
    pub fn thickness(mut self, thickness: Length) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// Returns `self` with the line thickness set to hairline, i.e. 1 device pixel.
    pub fn hairline(mut self) -> Self {
        self.thickness = None;
        self
    }

    /// Returns `self` with the given `dash_fit`.
    pub fn dash_fit(mut self, dash_fit: DashFit) -> Self {
        self.dash_fit = dash_fit;
        self
    }

    /// Returns `self` with the given `dash_pattern`.
    ///
    /// The pattern defines the lengths of dashes in alternating on/off order.
    /// * `10` - 10px dashes and 10px gaps
    /// * `10, 5` - 10px dashes with 5px gaps
    /// * `10, 5, 20, 30` - 10 px dash, 5px gap, 20px dash, 30px gap
    ///
    /// The pattern can be even longer and in any case will repeat to fill the whole divider space.
    ///
    /// The pattern must contain an even number of lengths. With exceptions for zero and one, where
    /// zero lengths means a solid line and one length will be used for both the dash and the gap.
    /// When given any other uneven number of the lengths, the last length will be ignored.
    ///
    /// # Panics
    ///
    /// Panics if `dash_pattern` contains an uneven number of entries of 3 or more
    /// and debug assertions are enabled.
    pub fn dash_pattern(mut self, dash_pattern: &[Length]) -> Self {
        let mut dash_pattern = SmallVec::from_slice(dash_pattern);
        // Paint code assumes an even number for simplicity of implementation.
        let len = dash_pattern.len();
        if len == 1 {
            dash_pattern.push(dash_pattern[0]);
        } else if len > 0 && !len.is_multiple_of(2) {
            debug_panic!(
                "The divider dash pattern must have an even number of lengths. Received {len}"
            );
            dash_pattern.pop();
        }
        self.dash_pattern = dash_pattern;
        self
    }

    /// Returns `self` with the given `cap` used both for start and end.
    ///
    /// Use [`start_cap`] or [`end_cap`] to set different edge caps.
    ///
    /// Defaults to [`Cap::Butt`].
    ///
    /// [`start_cap`]: Self::start_cap
    /// [`end_cap`]: Self::end_cap
    pub fn cap(mut self, cap: Cap) -> Self {
        self.start_cap = cap;
        self.end_cap = cap;
        self
    }

    /// Returns `self` with the given starting `cap`.
    ///
    /// Use [`cap`] to set the cap for both the start and the end.
    ///
    /// Defaults to [`Cap::Butt`].
    ///
    /// [`cap`]: Self::cap
    pub fn start_cap(mut self, cap: Cap) -> Self {
        self.start_cap = cap;
        self
    }

    /// Returns `self` with the given ending `cap`.
    ///
    /// Use [`cap`] to set the cap for both the start and the end.
    ///
    /// Defaults to [`Cap::Butt`].
    ///
    /// [`cap`]: Self::cap
    pub fn end_cap(mut self, cap: Cap) -> Self {
        self.end_cap = cap;
        self
    }

    /// Returns `self` with the given content `placement`.
    ///
    /// Defaults to [`Placement::Center`].
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Returns `self` with the given `pad`.
    ///
    /// This `pad` determines the amount of space between the divider line and the content.
    /// It does nothing when there is no content.
    ///
    /// The default value is 5px.
    pub fn pad(mut self, pad: Length) -> Self {
        self.pad = pad;
        self
    }
}

/// Use a distinctive number here, to be able to catch bugs.
/// In case the generational-id view path in `View::Message` leads to the wrong view.
/// This is a randomly generated 32 bit number - 2799496121 in decimal.
const DIVIDER_CONTENT_VIEW_ID: ViewId = ViewId::new(0xA6DCEBB9);

impl<State, Action, V> ViewMarker for Divider<State, Action, V> {}
impl<State: 'static, Action: 'static, V: WidgetView<State, Action>> View<State, Action, ViewCtx>
    for Divider<State, Action, V>
{
    type Element = Pod<widgets::Divider>;
    type ViewState = Option<V::ViewState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut divider = widgets::Divider::new(self.axis)
            .dash_fit(self.dash_fit)
            .dash_pattern(&self.dash_pattern)
            .start_cap(self.start_cap)
            .end_cap(self.end_cap)
            .placement(self.placement)
            .pad(self.pad);
        if let Some(thickness) = self.thickness {
            divider = divider.thickness(thickness);
        } else {
            divider = divider.hairline();
        }
        let mut view_state = None;
        if let Some(content) = &self.content {
            let (content, content_state) = ctx.with_id(DIVIDER_CONTENT_VIEW_ID, |ctx| {
                View::<State, Action, _>::build(content, ctx, app_state)
            });
            divider = divider.content(content.new_widget);
            view_state = Some(content_state);
        }
        (ctx.create_pod(divider), view_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.axis != self.axis {
            widgets::Divider::set_direction(&mut element, self.axis);
        }
        if prev.thickness != self.thickness {
            if let Some(thickness) = self.thickness {
                widgets::Divider::set_thickness(&mut element, thickness);
            } else {
                widgets::Divider::set_hairline(&mut element);
            }
        }
        if prev.dash_fit != self.dash_fit {
            widgets::Divider::set_dash_fit(&mut element, self.dash_fit);
        }
        if prev.dash_pattern != self.dash_pattern {
            widgets::Divider::set_dash_pattern(&mut element, &self.dash_pattern);
        }
        if prev.start_cap != self.start_cap {
            widgets::Divider::set_start_cap(&mut element, self.start_cap);
        }
        if prev.end_cap != self.end_cap {
            widgets::Divider::set_end_cap(&mut element, self.end_cap);
        }
        if prev.placement != self.placement {
            widgets::Divider::set_placement(&mut element, self.placement);
        }
        if prev.pad != self.pad {
            widgets::Divider::set_pad(&mut element, self.pad);
        }
        match (&prev.content, &self.content) {
            (Some(prev_content), Some(content)) => {
                ctx.with_id(DIVIDER_CONTENT_VIEW_ID, |ctx| {
                    View::<State, Action, _>::rebuild(
                        content,
                        prev_content,
                        view_state.as_mut().unwrap(),
                        ctx,
                        widgets::Divider::content_mut(&mut element)
                            .unwrap()
                            .downcast(),
                        app_state,
                    );
                });
            }
            (Some(_prev_content), None) => {
                widgets::Divider::remove_content(&mut element);
                *view_state = None;
            }
            (None, Some(content)) => {
                let (content, content_state) = ctx.with_id(DIVIDER_CONTENT_VIEW_ID, |ctx| {
                    View::<State, Action, _>::build(content, ctx, app_state)
                });
                widgets::Divider::set_content(&mut element, content.new_widget);
                *view_state = Some(content_state);
            }
            (None, None) => (),
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        if let Some(content) = &self.content {
            ctx.with_id(DIVIDER_CONTENT_VIEW_ID, |ctx| {
                View::<State, Action, _>::teardown(
                    content,
                    view_state.as_mut().unwrap(),
                    ctx,
                    widgets::Divider::content_mut(&mut element)
                        .unwrap()
                        .downcast(),
                );
            });
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(DIVIDER_CONTENT_VIEW_ID) => {
                if let Some(content) = &self.content {
                    content.message(
                        view_state.as_mut().unwrap(),
                        message,
                        widgets::Divider::content_mut(&mut element)
                            .unwrap()
                            .downcast(),
                        app_state,
                    )
                } else {
                    tracing::warn!("Got message for Divider content that no longer exists");
                    MessageResult::Stale
                }
            }
            None => {
                tracing::error!(
                    ?message,
                    "Message arrived for Divider, but Divider doesn't consume any messages, this is a bug."
                );
                MessageResult::Stale
            }
            _ => {
                tracing::warn!("Got unexpected id path in Divider::message");
                MessageResult::Stale
            }
        }
    }
}
