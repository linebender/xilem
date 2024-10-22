// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use tree_arena::ArenaMut;
use vello::kurbo::Affine;

use crate::app::{RenderRoot, RenderRootState};
use crate::core::{ComposeCtx, Widget, WidgetArenaMut, WidgetState};
use crate::passes::{enter_span_if, recurse_on_children};
use crate::util::AnyMap;

// --- MARK: RECURSE
fn compose_widget(
    global_state: &mut RenderRootState,
    widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    properties: ArenaMut<'_, AnyMap>,
    parent_transformed: bool,
    parent_window_transform: Affine,
) {
    let _span = enter_span_if(global_state.trace.compose, state.reborrow());
    let mut children = WidgetArenaMut {
        widget_children: widget.children,
        widget_state_children: state.children,
        properties_children: properties.children,
    };
    let widget = &mut **widget.item;
    let state = state.item;

    let transformed = parent_transformed || state.transform_changed;

    if !transformed && !state.needs_compose {
        return;
    }

    // the translation needs to be applied *after* applying the transform, as translation by scrolling should be within the transformed coordinate space. Same is true for the (layout) origin, to behave similar as in CSS.
    let local_translation = state.scroll_translation + state.origin.to_vec2();

    state.window_transform =
        parent_window_transform * state.transform.then_translate(local_translation);

    let local_rect = state.size.to_rect() + state.paint_insets;
    state.bounding_rect = state.window_transform.transform_rect_bbox(local_rect);

    let mut ctx = ComposeCtx {
        global_state,
        widget_state: state,
        children: children.reborrow_mut(),
    };
    if ctx.widget_state.request_compose {
        widget.compose(&mut ctx);
    }

    // We need to update the accessibility node's coordinates and repaint it at the new position.
    state.request_accessibility = true;
    state.needs_accessibility = true;
    state.needs_paint = true;

    state.needs_compose = false;
    state.request_compose = false;
    state.transform_changed = false;

    let id = state.id;
    let parent_transform = state.window_transform;
    let parent_state = state;
    recurse_on_children(id, widget, children, |widget, mut state, properties| {
        compose_widget(
            global_state,
            widget,
            state.reborrow_mut(),
            properties,
            transformed,
            parent_transform,
        );
        let parent_bounding_rect = parent_state.bounding_rect;

        if let Some(child_bounding_rect) = parent_state.clip_child(state.item.bounding_rect) {
            parent_state.bounding_rect = parent_bounding_rect.union(child_bounding_rect);
        }

        parent_state.merge_up(state.item);
    });
}

// --- MARK: ROOT
/// See the [passes documentation](../doc/05_pass_system.md#compose-pass).
pub(crate) fn run_compose_pass(root: &mut RenderRoot) {
    let _span = info_span!("compose").entered();

    // If widgets have moved, pointer-related info may be stale.
    // For instance, the "hovered" widget may have moved and no longer be under the pointer.
    if root.root_state().needs_compose {
        root.global_state.needs_pointer_pass = true;
    }

    let (root_widget, root_state, root_properties) = root.widget_arena.get_all_mut(root.root.id());
    compose_widget(
        &mut root.global_state,
        root_widget,
        root_state,
        root_properties,
        false,
        Affine::IDENTITY,
    );
}

// --- MARK: TESTS ---
#[cfg(test)]
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
