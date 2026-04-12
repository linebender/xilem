// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::{Span, trace_span};

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PaintLayerMode,
    PropertiesRef, RegisterCtx, Widget, WidgetId,
};
use crate::imaging::Painter;
use crate::kurbo::{Axis, Size};
use crate::layout::{LenReq, Length};

/// The preferred size of an unconstrained external surface.
const DEFAULT_LENGTH: Length = Length::const_px(100.);

/// A widget that reserves an in-tree slot for host-managed external content.
///
/// `ExternalSurface` participates in layout like a normal leaf widget, but Masonry does not
/// paint its contents into the retained `imaging` scene. Instead, it marks its subtree as an
/// external paint layer so hosts such as `masonry_winit` can realize it as a foreign surface,
/// 3D viewport, or compositor-managed layer.
///
/// Hosts discover these slots when they inspect the current visual layer plan during
/// `masonry_winit::app::AppDriver::present_visual_layers`.
#[derive(Default)]
pub struct ExternalSurface {
    alt_text: Option<ArcStr>,
}

impl ExternalSurface {
    /// Create a new external surface slot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the text that will describe the external surface to screen readers.
    ///
    /// If the surface is decorative, use `""`. If no useful text description is available,
    /// leave this unset.
    pub fn with_alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

impl Widget for ExternalSurface {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        match len_req {
            LenReq::FitContent(space) => space,
            _ => DEFAULT_LENGTH.get(),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // External content is expected to stay within the widget's content box.
        ctx.set_clip_path(size.to_rect());
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if let Some(alt_text) = &self.alt_text {
            node.set_description(&**alt_text);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn paint_layer_mode(&self) -> PaintLayerMode {
        PaintLayerMode::External
    }

    fn make_trace_span(&self, widget_id: WidgetId) -> Span {
        trace_span!("ExternalSurface", id = widget_id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        self.alt_text.as_ref().map(ToString::to_string)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::app::{RenderRoot, RenderRootOptions, WindowSizePolicy};
    use crate::core::{DefaultProperties, NewWidget, Widget, WidgetTag};
    use crate::dpi::PhysicalSize;
    use crate::kurbo::{Point, Rect, Size};
    use crate::layout::AsUnit;
    use crate::widgets::{ExternalSurface, Flex, SizedBox};

    #[test]
    fn marks_itself_as_external_layer() {
        let surface = ExternalSurface::new();
        assert_eq!(
            Widget::paint_layer_mode(&surface),
            crate::core::PaintLayerMode::External
        );
    }

    #[test]
    fn emits_external_layer_from_within_widget_tree() {
        let tag = WidgetTag::<ExternalSurface>::named("external-surface");
        let widget = Flex::row()
            .with_fixed(NewWidget::new(SizedBox::empty().size(20.0.px(), 20.0.px())))
            .with_fixed(NewWidget::new(ExternalSurface::new()).with_tag(tag))
            .with_auto_id();

        let mut render_root = RenderRoot::new(
            widget,
            |_| {},
            RenderRootOptions {
                default_properties: Arc::new(DefaultProperties::new()),
                use_system_fonts: false,
                size_policy: WindowSizePolicy::User,
                size: PhysicalSize::new(120, 40),
                scale_factor: 1.0,
                test_font: None,
            },
        );

        let (visual_layers, _) = render_root.redraw();
        let surface_ref = render_root.get_widget_with_tag(tag).unwrap();
        let external = visual_layers
            .external_layers()
            .map(|(_, layer)| layer)
            .next()
            .expect("missing external layer");

        assert_eq!(external.root_id, surface_ref.id());
        assert_eq!(
            external.transform.translation(),
            Point::new(20.0, 0.0).to_vec2()
        );
        assert_eq!(external.bounds.size(), Size::new(100.0, 40.0));
    }

    #[test]
    fn emits_external_layer_as_base_root() {
        let tag = WidgetTag::<ExternalSurface>::named("external-root");
        let mut render_root = RenderRoot::new(
            NewWidget::new(ExternalSurface::new().with_alt_text("viewport")).with_tag(tag),
            |_| {},
            RenderRootOptions {
                default_properties: Arc::new(DefaultProperties::new()),
                use_system_fonts: false,
                size_policy: WindowSizePolicy::User,
                size: PhysicalSize::new(80, 60),
                scale_factor: 1.0,
                test_font: None,
            },
        );

        let (visual_layers, _) = render_root.redraw();
        let surface_ref = render_root.get_widget_with_tag(tag).unwrap();
        let layers: Vec<_> = visual_layers
            .external_layers()
            .map(|(_, layer)| layer)
            .collect();

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].root_id, surface_ref.id());
        assert_eq!(layers[0].bounds.size(), Size::new(80.0, 60.0));
    }

    #[test]
    fn reports_window_bounds_for_centered_surface_slot() {
        let tag = WidgetTag::<ExternalSurface>::named("external-surface");
        let widget = Flex::column()
            .with_fixed(NewWidget::new(
                SizedBox::new(NewWidget::new(ExternalSurface::new()).with_tag(tag))
                    .size(280.0.px(), 140.0.px()),
            ))
            .with_auto_id();

        let mut render_root = RenderRoot::new(
            widget,
            |_| {},
            RenderRootOptions {
                default_properties: Arc::new(DefaultProperties::new()),
                use_system_fonts: false,
                size_policy: WindowSizePolicy::User,
                size: PhysicalSize::new(800, 600),
                scale_factor: 1.0,
                test_font: None,
            },
        );

        let (visual_layers, _) = render_root.redraw();
        let surface_ref = render_root.get_widget_with_tag(tag).unwrap();
        let external = visual_layers
            .external_layers()
            .map(|(_, layer)| layer)
            .find(|layer| layer.root_id == surface_ref.id())
            .expect("missing external layer");

        assert_eq!(
            external.window_bounds(),
            Rect::new(260.0, 0.0, 540.0, 140.0)
        );
    }
}
