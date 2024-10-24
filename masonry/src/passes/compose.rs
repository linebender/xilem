// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use vello::kurbo::Vec2;

use crate::passes::recurse_on_children;
use crate::render_root::{RenderRoot, RenderRootSignal, RenderRootState};
use crate::tree_arena::ArenaMut;
use crate::{ComposeCtx, Widget, WidgetState};

// --- MARK: RECURSE ---
fn compose_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    parent_moved: bool,
    parent_translation: Vec2,
) {
    let _span = global_state
        .trace
        .compose
        .then(|| widget.item.make_trace_span().entered());

    let moved = parent_moved || state.item.translation_changed;
    let translation = parent_translation + state.item.translation + state.item.origin.to_vec2();
    state.item.window_origin = translation.to_point();

    if !parent_moved && !state.item.translation_changed && !state.item.needs_compose {
        return;
    }

    let mut ctx = ComposeCtx {
        global_state,
        widget_state: state.item,
        widget_state_children: state.children.reborrow_mut(),
        widget_children: widget.children.reborrow_mut(),
    };
    if ctx.widget_state.request_compose {
        widget.item.compose(&mut ctx);
    }

    // TODO - Add unit tests for this.
    if moved && state.item.accepts_text_input && global_state.is_focused(state.item.id) {
        let ime_area = state.item.get_ime_area();
        global_state.emit_signal(RenderRootSignal::new_ime_moved_signal(ime_area));
    }

    // We need to update the accessibility node's coordinates and repaint it at the new position.
    state.item.request_accessibility = true;
    state.item.needs_accessibility = true;
    state.item.needs_paint = true;

    state.item.needs_compose = false;
    state.item.request_compose = false;
    state.item.translation_changed = false;

    let id = state.item.id;
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            compose_widget(
                global_state,
                widget,
                state.reborrow_mut(),
                moved,
                translation,
            );
            parent_state.merge_up(state.item);
        },
    );
}

// --- MARK: ROOT ---
pub(crate) fn run_compose_pass(root: &mut RenderRoot) {
    let _span = info_span!("compose").entered();

    // If widgets are moved, pointer-related info may be stale.
    // For instance, the "hovered" widget may have moved and no longer be under the pointer.
    if root.root_state().needs_compose {
        root.global_state.needs_pointer_pass = true;
    }

    let (root_widget, root_state) = root.widget_arena.get_pair_mut(root.root.id());
    compose_widget(
        &mut root.global_state,
        root_widget,
        root_state,
        false,
        Vec2::ZERO,
    );
}

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use smallvec::smallvec;
    use vello::kurbo::{Point, Size};

    use crate::testing::{
        widget_ids, ModularWidget, Record, Recording, TestHarness, TestWidgetExt as _,
    };
    use crate::widget::SizedBox;
    use crate::WidgetPod;

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
