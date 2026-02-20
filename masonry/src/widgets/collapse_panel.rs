// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, FromDynWidget, LayoutCtx, MeasureCtx, NewWidget, NoAction,
    PaintCtx, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Line, Point, Size};
use crate::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};
use crate::properties::{BorderColor, BorderWidth, Dimensions, Padding};
use crate::util::stroke;
use crate::vello::Scene;
use crate::widgets::{DisclosureButton, Label};
use crate::{accesskit, theme};

/// Square disclosure button length.
const BUTTON_LENGTH: Length = Length::const_px(16.);
/// Padding around the separator line.
const SEPARATOR_PAD: Padding = Padding {
    top: 4.,
    left: 1.,
    right: 1.,
    bottom: 0.,
};

/// A collapsible panel with a header that contains a child widget.
pub struct CollapsePanel<W: Widget + ?Sized> {
    disclosure_button: WidgetPod<DisclosureButton>,
    header_label: WidgetPod<Label>,
    /// The y location of the separator line.
    ///
    /// If it's [`None`], no line will be rendered.
    separator_line_y: Option<f64>,
    child: WidgetPod<W>,
}

impl<W: Widget + ?Sized> CollapsePanel<W> {
    /// Create a new [`CollapsePanel`] with a header text and a child widget.
    pub fn new(collapse: bool, header_text: impl Into<ArcStr>, child: NewWidget<W>) -> Self {
        Self {
            disclosure_button: Self::disclosure_button(collapse),
            header_label: WidgetPod::new(Label::new(header_text)),
            separator_line_y: None,
            child: child.to_pod(),
        }
    }

    /// Create a new [`CollapsePanel`] with a header label widget and a child widget.
    pub fn from_label(collapse: bool, header_label: NewWidget<Label>, child: NewWidget<W>) -> Self {
        Self {
            disclosure_button: Self::disclosure_button(collapse),
            header_label: header_label.to_pod(),
            separator_line_y: None,
            child: child.to_pod(),
        }
    }

    fn disclosure_button(collapse: bool) -> WidgetPod<DisclosureButton> {
        DisclosureButton::new(!collapse)
            .with_props(
                // TODO - Move to DefaultProperties
                Dimensions::fixed(BUTTON_LENGTH, BUTTON_LENGTH),
            )
            .to_pod()
    }
}

// --- MARK: WIDGETMUT
impl<W: Widget + FromDynWidget + ?Sized> CollapsePanel<W> {
    /// Set the child widget.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<W>) {
        this.ctx
            .remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
    }

    /// Set whether or not the panel is collapsed.
    pub fn set_collapsed(this: &mut WidgetMut<'_, Self>, collapsed: bool) {
        DisclosureButton::set_disclosed(&mut Self::disclosure_button_mut(this), !collapsed);
        this.ctx.request_layout();
    }

    /// Set the text.
    ///
    /// We enforce this to be an `ArcStr` to make the allocation explicit.
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: ArcStr) {
        Label::set_text(&mut Self::header_label_mut(this), new_text);
    }

    /// Get a mutable reference to the disclosure button.
    pub fn disclosure_button_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
    ) -> WidgetMut<'t, DisclosureButton> {
        this.ctx.get_mut(&mut this.widget.disclosure_button)
    }

    /// Get a mutable reference to the label.
    pub fn header_label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.header_label)
    }

    /// Get a mutable reference to the child.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, W> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

// --- MARK: IMPL WIDGET
impl<W: Widget + ?Sized> Widget for CollapsePanel<W> {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.disclosure_button);
        ctx.register_child(&mut self.header_label);
        ctx.register_child(&mut self.child);
    }

    fn property_changed(&mut self, _ctx: &mut UpdateCtx<'_>, _property_type: TypeId) {}

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let header_x_padding = theme::WIDGET_CONTROL_COMPONENT_PADDING;

        let header_x_padding_length = header_x_padding.dp(scale) * 2.;
        let btn_length = BUTTON_LENGTH.dp(scale);

        let separator_height =
            border.width * scale + SEPARATOR_PAD.length(Axis::Vertical).dp(scale);

        let space: LenDef = len_req.into();

        let cross = axis.cross();
        let label_cross_space = match cross {
            // If we know the horizontal space, then we can derive the label's horizontal space.
            Axis::Horizontal => cross_length
                .map(|cross_length| (cross_length - header_x_padding_length - btn_length).max(0.)),
            // Even if we know our vertical space, we don't know the child's height.
            // So we can't provide an accurate height for the label.
            Axis::Vertical => None,
        };
        // We don't give any special context to the label, just our full size
        let label_context_size = LayoutSize::maybe(cross, cross_length);
        let label_auto_length = match axis {
            Axis::Horizontal => space.reduce(header_x_padding_length + btn_length),
            Axis::Vertical => space,
        };
        let label_length = ctx.compute_length(
            &mut self.header_label,
            label_auto_length,
            label_context_size,
            axis,
            label_cross_space,
        );

        let header_length = match axis {
            Axis::Horizontal => btn_length + label_length + header_x_padding_length,
            Axis::Vertical => btn_length.max(label_length),
        };

        // Collapsed = !Disclosed
        let is_collapsed = !ctx.get_raw(&mut self.disclosure_button).0.is_disclosed();

        let child_length = if !is_collapsed {
            let child_cross_space = match cross {
                // If we know the horizontal space, then that is also the child's horizontal space.
                Axis::Horizontal => cross_length,
                // Even if we know our vertical space, we don't know the header's height.
                // So we can't provide an accurate height for the child.
                Axis::Vertical => None,
            };
            // Child's context size has the same restrictions as child's cross space.
            let child_context_size = LayoutSize::maybe(cross, child_cross_space);
            let child_auto_length = match axis {
                Axis::Horizontal => space,
                Axis::Vertical => space.reduce(header_length + separator_height),
            };
            ctx.compute_length(
                &mut self.child,
                child_auto_length,
                child_context_size,
                axis,
                child_cross_space,
            )
        } else {
            0.
        };

        match axis {
            Axis::Horizontal => header_length.max(child_length),
            Axis::Vertical => {
                let mut length = header_length;
                if !is_collapsed {
                    length += child_length + separator_height;
                }
                length
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let header_x_padding = theme::WIDGET_CONTROL_COMPONENT_PADDING;

        let separator_height =
            border.width * scale + SEPARATOR_PAD.length(Axis::Vertical).dp(scale);

        let button_width = BUTTON_LENGTH.dp(scale);
        let header_padding_width = header_x_padding.dp(scale);

        // Square button
        let button_size = Size::new(button_width, button_width);
        ctx.run_layout(&mut self.disclosure_button, button_size);

        let label_auto_size = SizeDef::new(
            LenDef::FitContent(
                (size.width - header_padding_width * 2. - button_size.width).max(0.),
            ),
            LenDef::FitContent(size.height),
        );
        let label_size = ctx.compute_size(&mut self.header_label, label_auto_size, size.into());

        ctx.run_layout(&mut self.header_label, label_size);

        let header_height = button_size.height.max(label_size.height);

        // Place it at the center of the label height.
        let btn_origin = Point::new(
            header_padding_width,
            (label_size.height - button_size.height) * 0.5,
        );
        ctx.place_child(&mut self.disclosure_button, btn_origin);

        let label_origin = Point::new(button_size.width + header_padding_width * 2.0, 0.0);
        ctx.place_child(&mut self.header_label, label_origin);

        // Collapsed = !Disclosed
        let is_collapsed = !ctx.get_raw(&mut self.disclosure_button).0.is_disclosed();

        // Only render child if it's not collapsed.
        ctx.set_stashed(&mut self.child, is_collapsed);

        if !is_collapsed {
            let child_space = Size::new(
                size.width,
                (size.height - header_height - separator_height).max(0.),
            );

            let child_auto_size = SizeDef::fit(child_space);
            let child_context_size = child_space.into();

            let child_size = ctx.compute_size(&mut self.child, child_auto_size, child_context_size);

            ctx.run_layout(&mut self.child, child_size);

            let child_origin = Point::new(0.0, header_height + separator_height);
            ctx.place_child(&mut self.child, child_origin);

            self.separator_line_y =
                Some(header_height + SEPARATOR_PAD.top * scale + border.width * scale * 0.5);
        } else {
            self.separator_line_y = None;
        }

        ctx.derive_baselines(&self.header_label);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        if let Some(y) = self.separator_line_y {
            let border_width = props.get::<BorderWidth>();
            let border_color = props.get::<BorderColor>();

            let border_box = ctx.border_box();

            // Only paint the line if it would have a positive width
            if SEPARATOR_PAD.length(Axis::Horizontal).dp(scale) < border_box.width() {
                let x1 = border_box.x0 + SEPARATOR_PAD.left * scale;
                let x2 = border_box.x1 - SEPARATOR_PAD.right * scale;
                let line = Line::new((x1, y), (x2, y));
                stroke(scene, &line, border_color.color, border_width.width);
            }
        }
    }

    fn accessibility_role(&self) -> accesskit::Role {
        accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[
            self.disclosure_button.id(),
            self.header_label.id(),
            self.child.id(),
        ])
    }
}
