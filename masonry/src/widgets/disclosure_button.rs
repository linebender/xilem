// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use core::any::TypeId;
use core::f64::consts::FRAC_PI_2;

use crate::accesskit;
use crate::core::keyboard::{Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NoAction, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
    WidgetMut,
};
use crate::kurbo::{Affine, Axis, BezPath, Join, Size, Stroke};
use crate::layout::{LenReq, Length};
use crate::palette::css::LIGHT_BLUE;
use crate::peniko::BrushRef;
use crate::properties::ContentColor;
use crate::util::stroke;
use crate::vello::Scene;

// Default size is a square
const DEFAULT_LENGTH: Length = Length::const_px(8.);

/// A triangle button that points towards the right when undisclosed
/// and points towards the bottom when disclosed.
#[derive(Default)]
pub struct DisclosureButton {
    is_disclosed: bool,
}

// --- MARK: BUILDERS
impl DisclosureButton {
    /// Create a new [`DisclosureButton`] with
    /// an initial disclosed state.
    pub fn new(is_disclosed: bool) -> Self {
        Self { is_disclosed }
    }

    /// Get the disclosed state of the button.
    #[inline]
    pub fn is_disclosed(&self) -> bool {
        self.is_disclosed
    }
}

// --- MARK: WIDGETMUT
impl DisclosureButton {
    /// Change the disclosed state of the button.
    pub fn set_disclosed(this: &mut WidgetMut<'_, Self>, is_disclosed: bool) {
        this.widget.is_disclosed = is_disclosed;

        this.ctx.request_layout();
    }

    #[inline]
    /// Switch the disclosed state of the button.
    /// True -> False | False -> True
    fn switch_disclosed_state(&mut self) {
        self.is_disclosed = !self.is_disclosed;
    }
}

// --- MARK: IMPL WIDGET
impl Widget for DisclosureButton {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down { .. } => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                    // Checked state impacts appearance and accessibility node
                    ctx.request_render();
                    // trace!("Checkbox {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::Up { .. } => {
                if ctx.is_pointer_capture_target() && ctx.is_hovered() && !ctx.is_disabled() {
                    self.switch_disclosed_state();
                    ctx.request_layout();

                    // TODO: Submit actions?
                    // ctx.submit_action(Action::CheckboxToggled(self.collapse));
                    // trace!("Checkbox {:?} released", ctx.widget_id());
                }
            }
            _ => (),
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(event) if event.state.is_up() => {
                if matches!(&event.key, Key::Character(c) if c == " ")
                    || event.key == Key::Named(NamedKey::Enter)
                {
                    self.switch_disclosed_state();
                    ctx.request_layout();

                    // TODO: Submit actions?
                    // ctx.submit_action(Action::ButtonPressed(None));
                }
            }
            _ => (),
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.target() != ctx.widget_id() {
            return;
        }

        if event.action == accesskit::Action::Click {
            self.switch_disclosed_state();
            ctx.request_layout();
            // TODO: Submit actions?
            // ctx.submit_action(Action::ButtonPressed(None));
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        ContentColor::prop_changed(ctx, property_type);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::HoveredChanged(_) | Update::FocusChanged(_)) {
            ctx.request_render();
        }
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let length = DEFAULT_LENGTH.dp(scale);

        match len_req {
            LenReq::MinContent | LenReq::MaxContent => length,
            LenReq::FitContent(space) => length.min(space),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;
        let button_color = props.get::<ContentColor>();

        let size = ctx.content_box_size();
        let half_size = size * 0.5;

        let mut arrow = BezPath::new();
        arrow.move_to((0.0, -half_size.height));
        arrow.line_to((half_size.width, 0.0));
        arrow.line_to((0.0, half_size.height));

        let mut affine = Affine::translate(half_size.to_vec2());

        // Rotate if it's disclosed
        if self.is_disclosed() {
            affine = affine.pre_rotate(FRAC_PI_2);
        }

        scene.stroke(
            &Stroke::new(2.0 * scale).with_join(Join::Miter),
            affine,
            BrushRef::Solid(button_color.color),
            None,
            &arrow,
        );

        if ctx.is_focus_target() {
            // TODO: Perhaps change the color of the arrow instead?
            let rect = ctx.border_box().to_rounded_rect(2.0);
            stroke(scene, &rect, BrushRef::Solid(LIGHT_BLUE), 1.0 * scale);
        }
    }

    fn accessibility_role(&self) -> accesskit::Role {
        accesskit::Role::DisclosureTriangle
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}
