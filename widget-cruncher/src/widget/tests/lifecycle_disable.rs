#[cfg(FALSE)]
#[test]
fn simple_disable() {
    const CHANGE_DISABLED: Selector<bool> = Selector::new("druid-tests.change-disabled");

    let test_widget_factory = |auto_focus: bool, id: WidgetId, state: Rc<Cell<Option<bool>>>| {
        ModularWidget::new(state)
            .lifecycle_fn(move |state, ctx, event, _, _| match event {
                LifeCycle::BuildFocusChain => {
                    if auto_focus {
                        ctx.register_for_focus();
                    }
                }
                LifeCycle::DisabledChanged(disabled) => {
                    state.set(Some(*disabled));
                }
                _ => {}
            })
            .event_fn(|_, ctx, event, _, _| {
                if let Event::Command(cmd) = event {
                    if let Some(disabled) = cmd.try_get(CHANGE_DISABLED) {
                        ctx.set_disabled(*disabled);
                    }
                }
            })
            .with_id(id)
    };

    let disabled_0: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_1: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_2: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_3: Rc<Cell<Option<bool>>> = Default::default();

    let check_states = |name: &str, desired: [Option<bool>; 4]| {
        if desired[0] != disabled_0.get()
            || desired[1] != disabled_1.get()
            || desired[2] != disabled_2.get()
            || desired[3] != disabled_3.get()
        {
            eprintln!(
                "test \"{}\":\nexpected: {:?}\n got:      {:?}",
                name,
                desired,
                [
                    disabled_0.get(),
                    disabled_1.get(),
                    disabled_2.get(),
                    disabled_3.get()
                ]
            );
            panic!();
        }
    };

    let id_0 = WidgetId::next();
    let id_1 = WidgetId::next();
    let id_2 = WidgetId::next();
    let id_3 = WidgetId::next();

    let root = Flex::row()
        .with_child(test_widget_factory(true, id_0, disabled_0.clone()))
        .with_child(test_widget_factory(true, id_1, disabled_1.clone()))
        .with_child(test_widget_factory(true, id_2, disabled_2.clone()))
        .with_child(test_widget_factory(true, id_3, disabled_3.clone()));

    Harness::create_simple((), root, |harness| {
        harness.send_initial_events();
        check_states("send_initial_events", [None, None, None, None]);
        assert_eq!(harness.window().focus_chain(), &[id_0, id_1, id_2, id_3]);
        harness.submit_command(CHANGE_DISABLED.with(true).to(id_0));
        check_states("Change 1", [Some(true), None, None, None]);
        assert_eq!(harness.window().focus_chain(), &[id_1, id_2, id_3]);
        harness.submit_command(CHANGE_DISABLED.with(true).to(id_2));
        check_states("Change 2", [Some(true), None, Some(true), None]);
        assert_eq!(harness.window().focus_chain(), &[id_1, id_3]);
        harness.submit_command(CHANGE_DISABLED.with(true).to(id_3));
        check_states("Change 3", [Some(true), None, Some(true), Some(true)]);
        assert_eq!(harness.window().focus_chain(), &[id_1]);
        harness.submit_command(CHANGE_DISABLED.with(false).to(id_2));
        check_states("Change 4", [Some(true), None, Some(false), Some(true)]);
        assert_eq!(harness.window().focus_chain(), &[id_1, id_2]);
        harness.submit_command(CHANGE_DISABLED.with(true).to(id_2));
        check_states("Change 5", [Some(true), None, Some(true), Some(true)]);
        assert_eq!(harness.window().focus_chain(), &[id_1]);
        //This is intended the widget should not receive an event!
        harness.submit_command(CHANGE_DISABLED.with(false).to(id_1));
        check_states("Change 6", [Some(true), None, Some(true), Some(true)]);
        assert_eq!(harness.window().focus_chain(), &[id_1]);
    })
}

#[cfg(FALSE)]
#[test]
fn disable_tree() {
    const MULTI_CHANGE_DISABLED: Selector<HashMap<WidgetId, bool>> =
        Selector::new("druid-tests.multi-change-disabled");

    let leaf_factory = |state: Rc<Cell<Option<bool>>>| {
        ModularWidget::new(state).lifecycle_fn(move |state, ctx, event, _, _| match event {
            LifeCycle::BuildFocusChain => {
                ctx.register_for_focus();
            }
            LifeCycle::DisabledChanged(disabled) => {
                state.set(Some(*disabled));
            }
            _ => {}
        })
    };

    let wrapper = |id: WidgetId, widget: Box<dyn Widget<()>>| {
        ModularWidget::new(WidgetPod::new(widget))
            .lifecycle_fn(|inner, ctx, event, data, env| {
                inner.lifecycle(ctx, event, data, env);
            })
            .event_fn(|inner, ctx, event, data, env| {
                if let Event::Command(cmd) = event {
                    if let Some(map) = cmd.try_get(MULTI_CHANGE_DISABLED) {
                        if let Some(disabled) = map.get(&ctx.widget_id()) {
                            ctx.set_disabled(*disabled);
                            return;
                        }
                    }
                }
                inner.event(ctx, event, data, env);
            })
            .with_id(id)
    };

    fn multi_update(states: &[(WidgetId, bool)]) -> Command {
        let payload = states.iter().cloned().collect::<HashMap<_, _>>();
        MULTI_CHANGE_DISABLED.with(payload).to(Target::Global)
    }

    let disabled_0: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_1: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_2: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_3: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_4: Rc<Cell<Option<bool>>> = Default::default();
    let disabled_5: Rc<Cell<Option<bool>>> = Default::default();

    let check_states = |name: &str, desired: [Option<bool>; 6]| {
        if desired[0] != disabled_0.get()
            || desired[1] != disabled_1.get()
            || desired[2] != disabled_2.get()
            || desired[3] != disabled_3.get()
            || desired[4] != disabled_4.get()
            || desired[5] != disabled_5.get()
        {
            eprintln!(
                "test \"{}\":\nexpected: {:?}\n got:      {:?}",
                name,
                desired,
                [
                    disabled_0.get(),
                    disabled_1.get(),
                    disabled_2.get(),
                    disabled_3.get(),
                    disabled_4.get(),
                    disabled_5.get()
                ]
            );
            panic!();
        }
    };

    let outer_id = WidgetId::next();
    let inner_id = WidgetId::next();
    let single_id = WidgetId::next();
    let root_id = WidgetId::next();

    let node0 = Flex::row()
        .with_child(leaf_factory(disabled_0.clone()))
        .with_child(leaf_factory(disabled_1.clone()))
        .boxed();

    let node1 = leaf_factory(disabled_2.clone()).boxed();

    let node2 = Flex::row()
        .with_child(wrapper(outer_id, wrapper(inner_id, node0).boxed()))
        .with_child(wrapper(single_id, node1))
        .with_child(leaf_factory(disabled_3.clone()))
        .with_child(leaf_factory(disabled_4.clone()))
        .with_child(leaf_factory(disabled_5.clone()))
        .boxed();

    let root = wrapper(root_id, node2);

    Harness::create_simple((), root, |harness| {
        harness.send_initial_events();
        check_states("Send initial events", [None, None, None, None, None, None]);
        assert_eq!(harness.window().focus_chain().len(), 6);

        harness.submit_command(multi_update(&[(root_id, true)]));
        check_states(
            "disable root (0)",
            [
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 0);
        harness.submit_command(multi_update(&[(inner_id, true)]));

        check_states(
            "disable inner (1)",
            [
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 0);

        // Node 0 should not be affected
        harness.submit_command(multi_update(&[(root_id, false)]));
        check_states(
            "enable root (2)",
            [
                Some(true),
                Some(true),
                Some(false),
                Some(false),
                Some(false),
                Some(false),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 4);

        // Changing inner and outer in different directions should not affect the leaves
        harness.submit_command(multi_update(&[(inner_id, false), (outer_id, true)]));
        check_states(
            "change inner outer (3)",
            [
                Some(true),
                Some(true),
                Some(false),
                Some(false),
                Some(false),
                Some(false),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 4);

        // Changing inner and outer in different directions should not affect the leaves
        harness.submit_command(multi_update(&[(inner_id, true), (outer_id, false)]));
        check_states(
            "change inner outer (4)",
            [
                Some(true),
                Some(true),
                Some(false),
                Some(false),
                Some(false),
                Some(false),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 4);

        // Changing two widgets on the same level
        harness.submit_command(multi_update(&[(single_id, true), (inner_id, false)]));
        check_states(
            "change horizontal (5)",
            [
                Some(false),
                Some(false),
                Some(true),
                Some(false),
                Some(false),
                Some(false),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 5);

        // Disabling the root should disable all widgets
        harness.submit_command(multi_update(&[(root_id, true)]));
        check_states(
            "disable root (6)",
            [
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 0);

        // Enabling a widget in a disabled tree should not affect the enclosed widgets
        harness.submit_command(multi_update(&[(single_id, false)]));
        check_states(
            "enable single (7)",
            [
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
                Some(true),
            ],
        );
        assert_eq!(harness.window().focus_chain().len(), 0);
    })
}
