// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

use std::cell::Cell;
use std::rc::Rc;

use smallvec::smallvec;

use crate::testing::{widget_ids, ModularWidget, TestHarness, TestWidgetExt as _};
use crate::widget::Flex;
use crate::*;

const CHANGE_DISABLED: Selector<bool> = Selector::new("masonry-test.change-disabled");

fn make_focusable_widget(id: WidgetId, state: Rc<Cell<Option<bool>>>) -> impl Widget {
    ModularWidget::new(state)
        .lifecycle_fn(move |state, ctx, event| match event {
            LifeCycle::BuildFocusChain => {
                ctx.register_for_focus();
            }
            LifeCycle::DisabledChanged(disabled) => {
                state.set(Some(*disabled));
            }
            _ => {}
        })
        .event_fn(|_, ctx, event| {
            if let Event::Command(cmd) = event {
                if let Some(disabled) = cmd.try_get(CHANGE_DISABLED) {
                    ctx.set_disabled(*disabled);
                }
            }
        })
        .with_id(id)
}

#[test]
fn simple_disable() {
    let disabled_event: Rc<Cell<Option<bool>>> = Default::default();
    let id_0 = WidgetId::next();
    let root = make_focusable_widget(id_0, disabled_event.clone());

    let mut harness = TestHarness::create(root);

    // Initial state: widget is enabled, no event received.
    assert_eq!(disabled_event.get(), None);
    assert!(!harness.get_widget(id_0).state().is_disabled());

    // Widget is set to enabled, but was already enabled: no DisabledChanged received.
    harness.submit_command(CHANGE_DISABLED.with(false).to(id_0));
    assert_eq!(disabled_event.get(), None);
    assert!(!harness.get_widget(id_0).state().is_disabled());

    // Widget is set to disabled, a DisabledChanged is received.
    harness.submit_command(CHANGE_DISABLED.with(true).to(id_0));
    assert_eq!(disabled_event.get(), Some(true));
    assert!(harness.get_widget(id_0).state().is_disabled());

    disabled_event.set(None);
    // Widget is set to disabled, but was already disabled: no DisabledChanged received.
    harness.submit_command(CHANGE_DISABLED.with(true).to(id_0));
    assert_eq!(disabled_event.get(), None);
    assert!(harness.get_widget(id_0).state().is_disabled());

    disabled_event.set(None);
    // Widget is set to enabled, a DisabledChanged is received.
    harness.submit_command(CHANGE_DISABLED.with(false).to(id_0));
    assert_eq!(disabled_event.get(), Some(false));
    assert!(!harness.get_widget(id_0).state().is_disabled());
}

#[test]
fn disable_tree() {
    fn make_parent_widget(id: WidgetId, child: impl Widget) -> impl Widget {
        ModularWidget::new(WidgetPod::new(child))
            .lifecycle_fn(|child, ctx, event| {
                child.lifecycle(ctx, event);
            })
            .event_fn(|child, ctx, event| {
                if let Event::Command(cmd) = event {
                    if let Some(disabled) = cmd.try_get(CHANGE_DISABLED) {
                        ctx.set_disabled(*disabled);
                        ctx.set_handled();
                        // TODO
                        //return;
                    }
                }
                child.on_event(ctx, event);
            })
            .layout_fn(|child, ctx, my_bc| {
                let size = child.layout(ctx, my_bc);
                ctx.place_child(child, Point::ZERO);
                size
            })
            .children_fn(|child| smallvec![child.as_dyn()])
            .with_id(id)
    }

    fn make_leaf_widget(id: WidgetId) -> impl Widget {
        make_focusable_widget(id, Default::default())
    }

    fn get_disabled_states(harness: &TestHarness, ids: &[WidgetId]) -> Vec<bool> {
        ids.iter()
            .map(|id| harness.get_widget(*id).state().is_disabled())
            .collect()
    }

    let [root_id, group_1_id, sub_group_1_id, group_2_id, leaf_1_id, leaf_2_id] = widget_ids();

    // Our widget hierarchy is:
    // - root
    //  - group_1
    //   - subgroup_1
    //    - leaf_1
    //  - group_2
    //   - leaf_2

    let root = make_parent_widget(
        root_id,
        Flex::row()
            .with_child(make_parent_widget(
                group_1_id,
                make_parent_widget(sub_group_1_id, make_leaf_widget(leaf_1_id)),
            ))
            .with_child(make_parent_widget(group_2_id, make_leaf_widget(leaf_2_id))),
    );

    let mut harness = TestHarness::create(root);

    // Initial state -> All widgets enabled
    assert_eq!(
        get_disabled_states(&harness, &[root_id, group_1_id, sub_group_1_id, group_2_id]),
        [false, false, false, false]
    );
    assert_eq!(
        get_disabled_states(&harness, &[leaf_1_id, leaf_2_id]),
        [false, false]
    );
    assert_eq!(harness.window().focus_chain().len(), 2);

    // Disable root -> All widgets disabled
    harness.submit_command(CHANGE_DISABLED.with(true).to(root_id));
    assert_eq!(
        get_disabled_states(&harness, &[root_id, group_1_id, sub_group_1_id, group_2_id]),
        [true, true, true, true]
    );
    assert_eq!(
        get_disabled_states(&harness, &[leaf_1_id, leaf_2_id]),
        [true, true]
    );
    assert_eq!(harness.window().focus_chain().len(), 0);

    // Disable group_1 -> All widgets still disabled
    harness.submit_command(CHANGE_DISABLED.with(true).to(group_1_id));
    assert_eq!(
        get_disabled_states(&harness, &[root_id, group_1_id, sub_group_1_id, group_2_id]),
        [true, true, true, true]
    );
    assert_eq!(
        get_disabled_states(&harness, &[leaf_1_id, leaf_2_id]),
        [true, true]
    );
    assert_eq!(harness.window().focus_chain().len(), 0);

    // Enable group_2 -> No effect
    harness.submit_command(CHANGE_DISABLED.with(false).to(group_2_id));
    assert_eq!(
        get_disabled_states(&harness, &[root_id, group_1_id, sub_group_1_id, group_2_id]),
        [true, true, true, true]
    );
    assert_eq!(
        get_disabled_states(&harness, &[leaf_1_id, leaf_2_id]),
        [true, true]
    );
    assert_eq!(harness.window().focus_chain().len(), 0);

    // Enable root -> Children of group_1 still disabled, all others enabled
    harness.submit_command(CHANGE_DISABLED.with(false).to(root_id));
    assert_eq!(
        get_disabled_states(&harness, &[root_id, group_1_id, sub_group_1_id, group_2_id]),
        [false, true, true, false]
    );
    assert_eq!(
        get_disabled_states(&harness, &[leaf_1_id, leaf_2_id]),
        [true, false]
    );
    assert_eq!(harness.window().focus_chain().len(), 1);
}
