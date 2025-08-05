// TODO - Compose tests
#[cfg(false)]
mod tests {
    use smallvec::smallvec;
    use vello::kurbo::{Point, Size};

    use crate::WidgetPod;
    use crate::testing::{
        ModularWidget, Record, Recording, TestHarness, TestWidgetExt as _, widget_ids,
    };
    use crate::widget::SizedBox;

    use super::*;

    #[test]
    fn test_compose_pass() {
        let record = Recording::default();
        let [parent_id, recorder_id] = widget_ids();
        let inner = SizedBox::new_with_id(SizedBox::empty().record(&record), recorder_id);
        let parent = ModularWidget::new((WidgetPod::new(inner), Point::ZERO, Vec2::ZERO))
            .layout_fn(|state, ctx, bc| {
                let (child, pos, _) = state;
                ctx.run_layout(child, bc);
                ctx.place_child(child, *pos);
                Size::ZERO
            })
            .compose_fn(|state, ctx| {
                let (child, _, translation) = state;
                ctx.set_child_translation(child, *translation);
            })
            .register_children_fn(move |state, ctx| {
                let (child, _, _) = state;
                ctx.register_child(child);
            })
            .children_fn(|(child, _, _)| smallvec![child.id()]);
        let root = SizedBox::new_with_id(parent, parent_id);

        let mut harness = TestHarness::create(root);
        record.clear();

        harness.edit_widget(parent_id, |mut widget| {
            // TODO - Find better way to express this
            let mut widget = widget.downcast::<ModularWidget<(WidgetPod<SizedBox>, Point, Vec2)>>();
            widget.widget.state.1 = Point::new(30., 30.);
            widget.ctx.request_layout();
        });
        assert_eq!(
            record.drain(),
            vec![
                Record::Layout(Size::new(400., 400.)),
                Record::Compose(Point::new(30., 30.)),
            ]
        );

        harness.edit_widget(parent_id, |mut widget| {
            // TODO - Find better way to express this
            let mut widget = widget.downcast::<ModularWidget<(WidgetPod<SizedBox>, Point, Vec2)>>();
            widget.widget.state.2 = Vec2::new(8., 8.);
            widget.ctx.request_compose();
        });

        // TODO - Should changing a parent transform call the child's compose method?
        assert_eq!(record.drain(), vec![]);
    }

    #[test]
    fn test_move_text_input() {
        let record = Recording::default();
        let [parent_id, recorder_id] = widget_ids();
        let inner = SizedBox::new_with_id(SizedBox::empty().record(&record), recorder_id);
        let parent = ModularWidget::new((WidgetPod::new(inner), Point::ZERO, Vec2::ZERO))
            .layout_fn(|state, ctx, bc| {
                let (child, pos, _) = state;
                ctx.run_layout(child, bc);
                ctx.place_child(child, *pos);
                Size::ZERO
            })
            .compose_fn(|state, ctx| {
                let (child, _, translation) = state;
                ctx.set_child_translation(child, *translation);
            })
            .register_children_fn(move |state, ctx| {
                let (child, _, _) = state;
                ctx.register_child(child);
            })
            .children_fn(|(child, _, _)| smallvec![child.id()]);
        let root = SizedBox::new_with_id(parent, parent_id);

        let mut harness = TestHarness::create(root);
        record.clear();

        harness.edit_widget(parent_id, |mut widget| {
            // TODO - Find better way to express this
            let mut widget = widget.downcast::<ModularWidget<(WidgetPod<SizedBox>, Point, Vec2)>>();
            widget.widget.state.1 = Point::new(30., 30.);
            widget.ctx.request_layout();
        });
        assert_eq!(
            record.drain(),
            vec![
                Record::Layout(Size::new(400., 400.)),
                Record::Compose(Point::new(30., 30.)),
            ]
        );

        harness.edit_widget(parent_id, |mut widget| {
            // TODO - Find better way to express this
            let mut widget = widget.downcast::<ModularWidget<(WidgetPod<SizedBox>, Point, Vec2)>>();
            widget.widget.state.2 = Vec2::new(8., 8.);
            widget.ctx.request_compose();
        });

        // TODO - Should changing a parent transform call the child's compose method?
        assert_eq!(record.drain(), vec![]);
    }
}
