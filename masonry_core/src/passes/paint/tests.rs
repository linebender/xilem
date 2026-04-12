// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use accesskit::{Node, Role};

use super::run_paint_pass;
use crate::app::{
    ExternalLayerKind, RenderRoot, RenderRootOptions, VisualLayer, VisualLayerBoundary,
    VisualLayerKind, VisualLayerPlan, WindowSizePolicy,
};
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, DefaultProperties, LayoutCtx, MeasureCtx, NewWidget,
    NoAction, PaintCtx, PaintLayerMode, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetPod,
};
use crate::dpi::PhysicalSize;
use crate::imaging::Painter;
use crate::imaging::record::Scene;
use crate::kurbo::{Axis, Point, Rect, Size};
use crate::layout::{LenReq, SizeDef};
use crate::peniko::Color;

/// Minimal leaf widget used to produce deterministic painted content with a selectable
/// `PaintLayerMode`.
struct PaintLeaf {
    color: Color,
    paint_layer_mode: PaintLayerMode,
}

impl Widget for PaintLeaf {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn on_pointer_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        if axis == Axis::Horizontal { 10.0 } else { 8.0 }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        painter.fill(ctx.content_box(), self.color).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn paint_layer_mode(&self) -> PaintLayerMode {
        self.paint_layer_mode
    }
}

/// Three fixed-width leaf widgets laid out in a row.
///
/// This is the basic fixture for assertions about how an isolated middle child splits the
/// ordered visual layer output around it.
struct TripleRow {
    left: WidgetPod<PaintLeaf>,
    middle: WidgetPod<PaintLeaf>,
    right: WidgetPod<PaintLeaf>,
}

impl Widget for TripleRow {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.left);
        ctx.register_child(&mut self.middle);
        ctx.register_child(&mut self.right);
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn on_pointer_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        if axis == Axis::Horizontal { 30.0 } else { 8.0 }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {
        let child_size = ctx.compute_size(
            &mut self.left,
            SizeDef::fixed(Size::new(10.0, 8.0)),
            Size::new(10.0, 8.0).into(),
        );
        ctx.run_layout(&mut self.left, child_size);
        ctx.place_child(&mut self.left, Point::new(0.0, 0.0));

        let child_size = ctx.compute_size(
            &mut self.middle,
            SizeDef::fixed(Size::new(10.0, 8.0)),
            Size::new(10.0, 8.0).into(),
        );
        ctx.run_layout(&mut self.middle, child_size);
        ctx.place_child(&mut self.middle, Point::new(10.0, 0.0));

        let child_size = ctx.compute_size(
            &mut self.right,
            SizeDef::fixed(Size::new(10.0, 8.0)),
            Size::new(10.0, 8.0).into(),
        );
        ctx.run_layout(&mut self.right, child_size);
        ctx.place_child(&mut self.right, Point::new(20.0, 0.0));
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        [self.left.id(), self.middle.id(), self.right.id()]
            .into_iter()
            .collect()
    }
}

/// Single-child wrapper that applies an offset and clip.
///
/// This exists to exercise ancestor transforms and clipping when a descendant becomes its own
/// visual layer.
struct OffsetBox {
    child: WidgetPod<PaintLeaf>,
    offset: Point,
    size: Size,
}

impl Widget for OffsetBox {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn on_pointer_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        if axis == Axis::Horizontal {
            self.size.width
        } else {
            self.size.height
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {
        ctx.set_clip_path(self.size.to_rect());
        let child_size =
            ctx.compute_size(&mut self.child, SizeDef::fixed(self.size), self.size.into());
        ctx.run_layout(&mut self.child, child_size);
        ctx.place_child(&mut self.child, self.offset);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        [self.child.id()].into_iter().collect()
    }
}

/// Variant of `TripleRow` whose middle slot is an `OffsetBox`.
///
/// This fixture is used for nested-boundary tests where an external layer must preserve
/// ancestor offset and clip information.
struct MixedTripleRow {
    left: WidgetPod<PaintLeaf>,
    middle: WidgetPod<OffsetBox>,
    right: WidgetPod<PaintLeaf>,
}

impl Widget for MixedTripleRow {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.left);
        ctx.register_child(&mut self.middle);
        ctx.register_child(&mut self.right);
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &Update,
    ) {
    }

    fn on_pointer_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn on_text_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &TextEvent,
    ) {
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut crate::core::EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        if axis == Axis::Horizontal { 30.0 } else { 8.0 }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {
        let child_size = ctx.compute_size(
            &mut self.left,
            SizeDef::fixed(Size::new(10.0, 8.0)),
            Size::new(10.0, 8.0).into(),
        );
        ctx.run_layout(&mut self.left, child_size);
        ctx.place_child(&mut self.left, Point::new(0.0, 0.0));

        let child_size = ctx.compute_size(
            &mut self.middle,
            SizeDef::fixed(Size::new(10.0, 8.0)),
            Size::new(10.0, 8.0).into(),
        );
        ctx.run_layout(&mut self.middle, child_size);
        ctx.place_child(&mut self.middle, Point::new(10.0, 0.0));

        let child_size = ctx.compute_size(
            &mut self.right,
            SizeDef::fixed(Size::new(10.0, 8.0)),
            Size::new(10.0, 8.0).into(),
        );
        ctx.run_layout(&mut self.right, child_size);
        ctx.place_child(&mut self.right, Point::new(20.0, 0.0));
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        [self.left.id(), self.middle.id(), self.right.id()]
            .into_iter()
            .collect()
    }
}

fn paint_result_for_middle(mode: PaintLayerMode) -> VisualLayerPlan {
    let root = TripleRow {
        left: NewWidget::new(PaintLeaf {
            color: Color::from_rgb8(255, 0, 0),
            paint_layer_mode: PaintLayerMode::Inline,
        })
        .to_pod(),
        middle: NewWidget::new(PaintLeaf {
            color: Color::from_rgb8(0, 255, 0),
            paint_layer_mode: mode,
        })
        .to_pod(),
        right: NewWidget::new(PaintLeaf {
            color: Color::from_rgb8(0, 0, 255),
            paint_layer_mode: PaintLayerMode::Inline,
        })
        .to_pod(),
    };
    let mut render_root = RenderRoot::new(
        NewWidget::new(root),
        |_| {},
        RenderRootOptions {
            default_properties: Arc::new(DefaultProperties::new()),
            use_system_fonts: false,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new(30, 8),
            scale_factor: 1.0,
            test_font: None,
        },
    );
    render_root.run_rewrite_passes();
    run_paint_pass(&mut render_root)
}

#[test]
fn replay_ignores_external_layers() {
    let base = Scene::new();
    let external = VisualLayer::external(
        ExternalLayerKind::Surface,
        VisualLayerBoundary::WidgetBoundary,
        Rect::ZERO,
        None,
        kurbo::Affine::IDENTITY,
        WidgetId::next(),
    );
    let result = VisualLayerPlan::new(vec![
        VisualLayer::scene(
            base,
            VisualLayerBoundary::LayerRoot,
            Rect::ZERO,
            None,
            kurbo::Affine::IDENTITY,
            WidgetId::next(),
        ),
        external,
    ]);

    let mut sink = Scene::new();
    result.replay_into(&mut sink);

    assert!(matches!(
        result.layers[1].kind,
        VisualLayerKind::External(ExternalLayerKind::Surface)
    ));
}

#[test]
fn isolated_scene_widget_splits_ordered_layers() {
    let result = paint_result_for_middle(PaintLayerMode::IsolatedScene);

    assert_eq!(result.layers.len(), 3);
    assert!(matches!(result.layers[0].kind, VisualLayerKind::Scene(_)));
    assert!(matches!(result.layers[1].kind, VisualLayerKind::Scene(_)));
    assert!(matches!(result.layers[2].kind, VisualLayerKind::Scene(_)));
    assert_eq!(result.layers[1].transform.translation(), (10.0, 0.0).into());
    assert_ne!(result.layers[0].root_id, result.layers[1].root_id);
    assert_eq!(result.layers[0].root_id, result.layers[2].root_id);
}

#[test]
fn external_widget_splits_ordered_layers() {
    let result = paint_result_for_middle(PaintLayerMode::External);

    assert_eq!(result.layers.len(), 3);
    assert!(matches!(result.layers[0].kind, VisualLayerKind::Scene(_)));
    assert!(matches!(
        result.layers[1].kind,
        VisualLayerKind::External(ExternalLayerKind::Surface)
    ));
    assert!(matches!(result.layers[2].kind, VisualLayerKind::Scene(_)));
    assert_eq!(result.layers[1].transform.translation(), (10.0, 0.0).into());
}

#[test]
fn nested_external_widget_preserves_ancestor_offsets() {
    let root = MixedTripleRow {
        left: NewWidget::new(PaintLeaf {
            color: Color::from_rgb8(255, 0, 0),
            paint_layer_mode: PaintLayerMode::Inline,
        })
        .to_pod(),
        middle: NewWidget::new(OffsetBox {
            child: NewWidget::new(PaintLeaf {
                color: Color::from_rgb8(0, 255, 0),
                paint_layer_mode: PaintLayerMode::External,
            })
            .to_pod(),
            offset: Point::new(7.0, 5.0),
            size: Size::new(10.0, 8.0),
        })
        .to_pod(),
        right: NewWidget::new(PaintLeaf {
            color: Color::from_rgb8(0, 0, 255),
            paint_layer_mode: PaintLayerMode::Inline,
        })
        .to_pod(),
    };
    let mut render_root = RenderRoot::new(
        NewWidget::new(root),
        |_| {},
        RenderRootOptions {
            default_properties: Arc::new(DefaultProperties::new()),
            use_system_fonts: false,
            size_policy: WindowSizePolicy::User,
            size: PhysicalSize::new(30, 8),
            scale_factor: 1.0,
            test_font: None,
        },
    );
    render_root.run_rewrite_passes();
    let result = run_paint_pass(&mut render_root);

    assert_eq!(result.layers.len(), 3);
    assert!(matches!(
        result.layers[1].kind,
        VisualLayerKind::External(ExternalLayerKind::Surface)
    ));
    assert_eq!(result.layers[1].transform.translation(), (17.0, 5.0).into());
    assert_eq!(result.layers[1].bounds, Rect::new(0.0, 0.0, 10.0, 8.0));
    assert_eq!(result.layers[1].clip, None);
    assert_eq!(
        result.layers[1].window_bounds(),
        Rect::new(17.0, 5.0, 27.0, 13.0)
    );
}
