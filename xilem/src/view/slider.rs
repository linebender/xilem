// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::Slider as MasonrySlider;
use xilem_core::{MessageResult, View, ViewCtx};

use crate::{Pod, WidgetView};

/// A slider widget for selecting a value within a range.
///
/// # Example
/// ```rust
/// use xilem::view::slider;
///
/// slider(0.5, |value| println!("Slider value: {}", value));
/// ```
pub fn slider<State, Action>(
    value: f64,
    on_change: impl FnMut(f64) -> Action + Send + Sync + 'static,
) -> Slider<State, Action> {
    Slider {
        value,
        on_change: Box::new(on_change),
    }
}

/// A slider view that allows selecting a value within a range.
pub struct Slider<State, Action> {
    value: f64,
    on_change: Box<dyn FnMut(f64) -> Action + Send + Sync>,
}

impl<State, Action> View<State, Action, ViewCtx> for Slider<State, Action> {
    type Element = Pod<MasonrySlider>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Pod<MasonrySlider>, ()) {
        let slider = MasonrySlider::new(self.value)
            .on_change(move |value| {
                let action = (self.on_change)(value);
                ctx.proxy().send(action).unwrap();
            });
        (ctx.new_pod(slider), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        mut element: xilem_core::Mut<Self::Element>,
        _ctx: &mut ViewCtx,
        _state: &mut Self::ViewState,
    ) -> MessageResult<Action> {
        element.with_downcast_mut(|slider| {
            slider.set_value(self.value);
        });
        MessageResult::Nop
    }
}

impl<State, Action> WidgetView<State, Action> for Slider<State, Action> {
    type Widget = MasonrySlider;
}
