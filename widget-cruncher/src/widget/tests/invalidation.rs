// Copyright 2020 The Druid Authors.
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

//! Tests related to propagation of invalid rects.

use crate::testing::{move_mouse, widget_ids, Harness};
use crate::widget::{Button, Flex};
use test_env_log::test;

#[test]
fn invalidate_union() {
    let [id_child_1, id_child_2] = widget_ids();

    let widget = Flex::column()
        .with_child_id(Button::new("hi"), id_child_1)
        .with_child_id(Button::new("there"), id_child_2);

    let mut harness = Harness::create(widget);

    let child1_rect = harness.get_widget(id_child_1).state().layout_rect();
    let child2_rect = harness.get_widget(id_child_2).state().layout_rect();
    harness.mouse_move_to(id_child_1);
    assert_eq!(harness.window().invalid().rects(), &[child1_rect]);

    // This resets the invalid region.
    let _ = harness.render();
    assert!(harness.window().invalid().is_empty());

    harness.mouse_move_to(id_child_2);
    assert_eq!(
        harness.window().invalid().rects(),
        // TODO: this is probably too fragile, because is there any guarantee on the order?
        &[child1_rect, child2_rect]
    );
}

#[cfg(FALSE)]
#[test]
fn invalidate_scroll() {
    const RECT: Rect = Rect {
        x0: 30.,
        y0: 40.,
        x1: 40.,
        y1: 50.,
    };

    struct Invalidator;

    impl Widget for Invalidator {
        fn on_event(&mut self, ctx: &mut EventCtx, _: &Event, _: &Env) {
            ctx.request_paint_rect(RECT);
        }

        fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &Env) {}
        fn layout(&mut self, _: &mut LayoutCtx, _: &BoxConstraints, _: &Env) -> Size {
            Size::new(1000., 1000.)
        }

        fn paint(&mut self, ctx: &mut PaintCtx, _: &Env) {
            use float_cmp::approx_eq;

            assert_eq!(ctx.region().rects().len(), 1);
            let rect = ctx.region().rects().first().unwrap();

            approx_eq!(f64, rect.x0, RECT.x0);
            approx_eq!(f64, rect.y0, RECT.y0);
            approx_eq!(f64, rect.x1, RECT.x1);
            approx_eq!(f64, rect.y1, RECT.y1);
        }
    }

    let id = WidgetId::next();
    let scroll_id = WidgetId::next();
    let invalidator = IdentityWrapper::wrap(Invalidator, id);
    let scroll = Scroll::new(invalidator).with_id(scroll_id);

    let mut harness = Harness::create(scroll);

    // Sending an event should cause RECT to get invalidated.
    harness.event(Event::MouseMove(move_mouse((10., 10.))));
    assert_eq!(harness.window().invalid().rects(), &[RECT]);

    // This resets the invalid region, and our widget checks to make sure it sees the right
    // invalid region in the paint function.
    harness.paint_invalid();
    assert!(harness.window().invalid().is_empty());

    harness.event(Event::Wheel(scroll_mouse((10., 10.), (7.0, 9.0))));
    // Scrolling invalidates the whole window.
    assert_eq!(
        harness.window().invalid().rects(),
        &[Size::new(400., 400.).to_rect()]
    );
    harness.window_mut().invalid_mut().clear();

    // After the scroll, the window should see the translated invalid regions...
    harness.event(Event::MouseMove(move_mouse((10., 10.))));
    assert_eq!(
        harness.window().invalid().rects(),
        &[RECT - Vec2::new(7.0, 9.0)]
    );
    // ...but in its paint callback, the widget will see the invalid region relative to itself.
    harness.paint_invalid();
}

// TODO: Add a test with scrolling
