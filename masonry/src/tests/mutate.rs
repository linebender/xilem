// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::mpsc;

use masonry_testing::{ModularWidget, TestHarness};

use crate::core::{NewWidget, WidgetTag};
use crate::theme::test_property_set;
use crate::widgets::SizedBox;

#[test]
fn mutate_order() {
    let parent_tag = WidgetTag::named("parent");
    let child = NewWidget::new(SizedBox::empty());
    let parent = NewWidget::new_with_tag(ModularWidget::new_parent(child), parent_tag);
    let grandparent = NewWidget::new(ModularWidget::new_parent(parent));

    let (sender, receiver) = mpsc::channel::<u32>();
    let sender1 = sender.clone();
    let sender2 = sender.clone();
    let sender3 = sender;

    let mut harness = TestHarness::create(test_property_set(), grandparent);
    harness.edit_widget(parent_tag, move |mut parent| {
        parent.ctx.mutate_self_later(move |_| {
            sender2.send(2).unwrap();
        });

        parent
            .ctx
            .mutate_child_later(&mut parent.widget.state, move |_| {
                sender3.send(3).unwrap();
            });

        sender1.send(1).unwrap();
    });

    let values: Vec<_> = receiver.iter().collect();
    assert_eq!(values, [1, 2, 3]);
}

#[test]
fn cancel_mutate() {
    let parent_tag = WidgetTag::named("parent");
    let child = NewWidget::new(SizedBox::empty());
    let parent = NewWidget::new_with_tag(SizedBox::new(child), parent_tag);

    let mut harness = TestHarness::create(test_property_set(), parent);
    harness.edit_widget(parent_tag, move |mut parent| {
        {
            let mut child = SizedBox::child_mut(&mut parent).unwrap();

            child.ctx.mutate_self_later(move |_| {
                panic!("This function should never get called");
            });
        }

        SizedBox::remove_child(&mut parent);
    });
}
