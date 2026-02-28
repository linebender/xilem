// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use assert_matches::assert_matches;

use crate::core::{ChildrenIds, Widget};
use crate::kurbo::Point;
use crate::layout::{AsUnit, LayoutSize};
use crate::properties::Dimensions;
use crate::testing::{ModularWidget, TestHarness};
use crate::theme::test_property_set;
use crate::widgets::{Button, ButtonPress};

/// This test covers two things:
///
/// 1. Nothing should crash if the action source is deleted before the action is propagated.
/// 2. The action should not be propagated at all if the source was deleted before propagation.
#[test]
fn action_source_removed() {
    let ok = Arc::new(AtomicBool::new(false));

    #[derive(Debug)]
    struct ArbitraryAction;

    let action_source = ModularWidget::new(ok.clone())
        .pointer_event_fn(|ok, ctx, _, _| {
            // Send an action but crucially don't mark the pointer event as handled,
            // so that we can also react to it in the parent widget.
            ctx.submit_untyped_action(Box::new(ArbitraryAction));
            ok.store(true, Ordering::Release);
        })
        .with_props(Dimensions::fixed(50.px(), 50.px()))
        .to_pod();
    let action_source_id = action_source.id();

    let parent = ModularWidget::new(Some(action_source))
        .pointer_event_fn(|child, ctx, _, _| {
            if let Some(child) = child.take() {
                // Remove the child immediately after it submits the action,
                // but before we've had a chance to react to the action.
                ctx.remove_child(child);
            }
        })
        .action_fn(|_, _, _, _, _| {
            // We don't expect the action to arrive because we have already deleted the child.
            panic!("Unexpected action");
        })
        .register_children_fn(|child, ctx| {
            if let Some(child) = child {
                ctx.register_child(child);
            }
        })
        .measure_fn(move |child, ctx, _props, axis, len_req, cross_length| {
            if let Some(child) = child {
                let auto_length = len_req.into();
                let context_size = LayoutSize::maybe(axis.cross(), cross_length);

                ctx.compute_length(child, auto_length, context_size, axis, cross_length)
            } else {
                0.
            }
        })
        .layout_fn(move |child, ctx, _props, size| {
            if let Some(child) = child {
                ctx.run_layout(child, size);
                ctx.place_child(child, Point::ZERO);
            }
        })
        .children_fn(|child| {
            let mut ids = ChildrenIds::new();
            if let Some(child) = child {
                ids.push(child.id());
            }
            ids
        })
        .with_auto_id();

    let mut harness = TestHarness::create(test_property_set(), parent);

    harness.mouse_move_to(action_source_id);

    // We don't expect the action to make it to the app driver,
    // because we deleted the child before it got there.
    assert_matches!(harness.pop_action::<ArbitraryAction>(), None);

    assert!(ok.load(Ordering::Acquire), "pointer event didn't happen");
}

#[test]
fn action_propagation() {
    #[derive(Debug)]
    struct TranslatedAction;

    let button = Button::with_text("Click me!").with_auto_id();
    let button_id = button.id();

    let parent1 = ModularWidget::new_parent(button)
        .action_fn(move |_, _, _, action, source| {
            // We expect only the button press action
            assert_eq!(source, button_id, "unexpected action source");
            assert!(action.is::<ButtonPress>(), "unexpected action type");
        })
        .with_auto_id();

    let parent2 = ModularWidget::new_parent(parent1)
        .action_fn(move |_, ctx, _, action, source| {
            // We expect only the button press action
            assert_eq!(source, button_id, "unexpected action source");
            assert!(action.is::<ButtonPress>(), "unexpected action type");
            // Mark the button press as handled to stop its propagation
            ctx.set_handled();
            // Translate it into our own action
            ctx.submit_untyped_action(Box::new(TranslatedAction));
        })
        .with_auto_id();
    let parent2_id = parent2.id();

    let parent3 = ModularWidget::new_parent(parent2)
        .action_fn(move |_, _, _, action, source| {
            // We expect only the translated action
            assert_eq!(source, parent2_id, "unexpected action source");
            assert!(action.is::<TranslatedAction>(), "unexpected action type");
        })
        .with_auto_id();

    let mut harness = TestHarness::create(test_property_set(), parent3);

    harness.mouse_click_on(button_id);

    // Only the translated action should reach the app driver
    assert_matches!(
        harness.pop_action::<TranslatedAction>(),
        Some((TranslatedAction, _))
    );

    // The button press should not reach the app driver
    assert_matches!(harness.pop_action_erased(), None);
}
