// Copyright 2022 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{Widget, WidgetPod};
use crate::testing::{ModularWidget, TestHarness};
use crate::theme::default_property_set;
use crate::widgets::Flex;

fn make_parent_widget<W: Widget>(child: W) -> ModularWidget<WidgetPod<W>> {
    ModularWidget::new_parent(child.with_auto_id())
}

#[should_panic(expected = "did not call RegisterCtx::register_child()")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_forget_register_child() {
    let widget = make_parent_widget(Flex::row())
        .register_children_fn(|_child, _ctx| {
            // We forget to call ctx.register_child();
        })
        .with_auto_id();

    let _harness = TestHarness::create(default_property_set(), widget);
}

#[should_panic(expected = "in the list returned by children_ids")]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "This test doesn't work without debug assertions (i.e. in release mode). See https://github.com/linebender/xilem/issues/477"
)]
fn check_register_invalid_child() {
    let widget = make_parent_widget(Flex::row())
        .register_children_fn(|child, ctx| {
            ctx.register_child(child);
            ctx.register_child(&mut WidgetPod::new(Flex::row()));
        })
        .with_auto_id();

    let _harness = TestHarness::create(default_property_set(), widget);
}
