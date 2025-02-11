// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A button widget with up to 9 labels.

use accesskit::{Node, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, trace_span, Span};
use vello::Scene;

use crate::core::{
    self, AccessCtx, AccessEvent, Action, ArcStr, BoxConstraints, EventCtx, LayoutCtx, PaintCtx,
    PointerButton, PointerEvent, QueryCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::kurbo::{Insets, Size, Vec2};
use crate::theme;
use crate::util::{fill_lin_gradient, stroke, UnitPoint};
use crate::widgets::Label;

/// The minimum padding added to a button. NOTE: these values are chosen to match the existing look of TextBox; these should be reevaluated at some point.
pub const PAD_DEF: Insets = Insets::uniform_xy(8., 2.);

/// A button with up to 9 text Labels (allowing for custom styles) with custom padding
/// (allowing for flexible positioning).
pub struct Button9 {
    /// 9 label widgets
    label: Label9,
    /// Options for those widgets or the button as a whole (only padding is implemented)
    opt: LabelOpt,
}
/// Label widgets for Button9 for all the 9 possible label positions in a button from top left to bottom right.<br>
/// 1 2 3 = ↖  ↑  ↗
/// 4 5 6 = ←  •  →
/// 7 8 9 = ↙  ↓  ↘
pub struct Label9 {
    p1: WidgetPod<Label>,
    p2: WidgetPod<Label>,
    p3: WidgetPod<Label>, // ↖  ↑  ↗
    p4: WidgetPod<Label>,
    p5: WidgetPod<Label>,
    p6: WidgetPod<Label>, // ←  •  →
    p7: WidgetPod<Label>,
    p8: WidgetPod<Label>,
    p9: WidgetPod<Label>, // ↙  ↓  ↘
}
/// Custom button options. Currently only padding is supported.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct LabelOpt {
    /// Per-label padding.
    pub pad: Pad9,
}

/// Optional padding options per label as [`Insets`]
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Pad9 {
    pub p1: Option<Insets>,
    pub p2: Option<Insets>,
    pub p3: Option<Insets>, // ↖  ↑  ↗
    pub p4: Option<Insets>,
    pub p5: Option<Insets>,
    pub p6: Option<Insets>, // ←  •  →
    pub p7: Option<Insets>,
    pub p8: Option<Insets>,
    pub p9: Option<Insets>, // ↙  ↓  ↘
}

// --- MARK: BUILDERS ---
impl Button9 {
    /// Create a new button with a text label at the center (p5 other labels are blank, use `.addx` methods to fill them)
    /// ```
    /// use masonry::widgets::Button9;
    /// let button = Button9::new("Increment");
    /// ```
    pub fn new(text: impl Into<ArcStr>) -> Self {
        Self::from_label(Label::new(text))
    }
    /// Create a new button with the provided [`Label`]
    /// ```
    /// use masonry::peniko::Color;
    /// use masonry::widgets::{Button9, Label};
    /// let label = Label::new("Increment").with_brush(Color::new([0.5, 0.5, 0.5, 1.0]));
    /// let button = Button9::from_label(label);
    /// ```
    pub fn from_label(label: Label) -> Self {
        Self::from_label_pad(label, None)
    }
    /// Create a new button with the provided [`Label`] and padding [`Insets`]
    /// ```
    /// use masonry::peniko::Color;
    /// use masonry::widgets::{Button9, Label};
    /// let label  = Label::new("Increment").with_brush(Color::new([0.5, 0.5, 0.5, 1.0]));
    /// let pad    = Insets::uniform_xy(8., 2.); // pad ←→ by 8 and ↑↓ by 2
    /// let button = Button9::from_label_pad(label, pad);
    /// ```
    pub fn from_label_pad(lbl: Label, pad: Option<Insets>) -> Self {
        let label = Label9 {
            p1: WidgetPod::new(Label::new("")),
            p2: WidgetPod::new(Label::new("")),
            p3: WidgetPod::new(Label::new("")), // ↖  ↑  ↗
            p4: WidgetPod::new(Label::new("")),
            p5: WidgetPod::new(lbl),
            p6: WidgetPod::new(Label::new("")), // ←  •  →
            p7: WidgetPod::new(Label::new("")),
            p8: WidgetPod::new(Label::new("")),
            p9: WidgetPod::new(Label::new("")), // ↙  ↓  ↘
        };
        let pad = Pad9 {
            p1: None,
            p2: None,
            p3: None, // ↖  ↑  ↗
            p4: None,
            p5: pad,
            p6: None, // ←  •  →
            p7: None,
            p8: None,
            p9: None, // ↙  ↓  ↘
        };
        let opt = LabelOpt { pad };
        Self { label, opt }
    }
    /// Helper .methods for adding individual labels (add=center p5)
    pub fn add(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p5 = WidgetPod::new(label);
        self.opt.pad.p5 = pad;
        self
    }
    pub fn add1(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p1 = WidgetPod::new(label);
        self.opt.pad.p1 = pad;
        self
    }
    pub fn add2(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p2 = WidgetPod::new(label);
        self.opt.pad.p2 = pad;
        self
    }
    pub fn add3(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p3 = WidgetPod::new(label);
        self.opt.pad.p3 = pad;
        self
    }
    pub fn add4(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p4 = WidgetPod::new(label);
        self.opt.pad.p4 = pad;
        self
    }
    pub fn add5(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p5 = WidgetPod::new(label);
        self.opt.pad.p5 = pad;
        self
    }
    pub fn add6(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p6 = WidgetPod::new(label);
        self.opt.pad.p6 = pad;
        self
    }
    pub fn add7(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p7 = WidgetPod::new(label);
        self.opt.pad.p7 = pad;
        self
    }
    pub fn add8(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p8 = WidgetPod::new(label);
        self.opt.pad.p8 = pad;
        self
    }
    pub fn add9(mut self, label: Label, pad: Option<Insets>) -> Self {
        self.label.p9 = WidgetPod::new(label);
        self.opt.pad.p9 = pad;
        self
    }
    /// Create a new button with the provided [`Label9`]s and their [`Pad9`] with predetermined IDs. This constructor is useful for toolkits which use Masonry (such as Xilem).
    pub fn from_label_pod(label_l: [WidgetPod<Label>; 9], pad: Pad9) -> Self {
        let [l1, l2, l3, l4, l5, l6, l7, l8, l9] = label_l;
        let label = Label9 {
            //numbering shifted due to 0-based array index
            p1: l1,
            p2: l2,
            p3: l3, // ↖  ↑  ↗
            p4: l4,
            p5: l5,
            p6: l8, // ←  •  →
            p7: l7,
            p8: l6,
            p9: l9, // ↙  ↓  ↘
        };
        let opt = LabelOpt { pad };
        Self { label, opt }
    }
}

// Helper indices for the Label9 positions (0-based unlike .Prop or fn() names!)
const ROW_TOP: [usize; 3] = [0, 1, 2]; //↖ ↑ ↗
const ROW_MID: [usize; 3] = [3, 4, 5]; //← • →
const ROW_BOT: [usize; 3] = [6, 7, 8]; //↙ ↓ ↘
const COL_LHS: [usize; 3] = [0, 1, 2]; //↖ ← ↙
const COL_CNT: [usize; 3] = [3, 4, 5]; //↑ • ↓
const COL_RHS: [usize; 3] = [6, 7, 8]; //↗ → ↘

// --- MARK: WIDGETMUT ---
impl Button9 {
    /// Set text helpers
    pub fn set_text(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label_mut(this), new_text);
    }
    pub fn set_text1(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label1_mut(this), new_text);
    }
    pub fn set_text2(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label2_mut(this), new_text);
    }
    pub fn set_text3(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label3_mut(this), new_text);
    }
    pub fn set_text4(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label4_mut(this), new_text);
    }
    pub fn set_text5(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label5_mut(this), new_text);
    }
    pub fn set_text6(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label6_mut(this), new_text);
    }
    pub fn set_text7(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label7_mut(this), new_text);
    }
    pub fn set_text8(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label8_mut(this), new_text);
    }
    pub fn set_text9(this: &mut WidgetMut<'_, Self>, new_text: impl Into<ArcStr>) {
        Label::set_text(&mut Self::label9_mut(this), new_text);
    }

    /// Set label options helpers
    pub fn set_opt(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p5 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad1(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p1 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad2(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p2 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad3(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p3 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad4(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p4 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad5(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p5 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad6(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p6 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad7(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p7 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad8(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p8 = new_pad;
        this.ctx.request_render();
    }
    pub fn set_pad9(this: &mut WidgetMut<'_, Self>, new_pad: Option<Insets>) {
        this.widget.opt.pad.p9 = new_pad;
        this.ctx.request_render();
    }

    /// Get mutable label helpers
    pub fn label_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p5)
    }
    pub fn label1_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p1)
    }
    pub fn label2_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p2)
    }
    pub fn label3_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p3)
    }
    pub fn label4_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p4)
    }
    pub fn label5_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p5)
    }
    pub fn label6_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p6)
    }
    pub fn label7_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p7)
    }
    pub fn label8_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p8)
    }
    pub fn label9_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.label.p9)
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for Button9 {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        match event {
            PointerEvent::PointerDown(_, _) => {
                if !ctx.is_disabled() {
                    ctx.capture_pointer();
                    ctx.request_paint_only(); // Changes in pointer capture impact appearance, but not accessibility node
                    trace!("Button9 {:?} pressed", ctx.widget_id());
                }
            }
            PointerEvent::PointerUp(button, _) => {
                if ctx.is_pointer_capture_target() && ctx.is_hovered() && !ctx.is_disabled() {
                    ctx.submit_action(Action::ButtonPressed(*button));
                    trace!("Button9 {:?} released", ctx.widget_id());
                }
                ctx.request_paint_only(); // Changes in pointer capture impact appearance, but not accessibility node
            }
            _ => (),
        }
    }

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if ctx.target() == ctx.widget_id() {
            match event.action {
                accesskit::Action::Click => {
                    ctx.submit_action(Action::ButtonPressed(PointerButton::Primary));
                }
                _ => {}
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) {
        match event {
            Update::HoveredChanged(_) | Update::FocusChanged(_) | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut core::RegisterCtx) {
        ctx.register_child(&mut self.label.p1);
        ctx.register_child(&mut self.label.p2);
        ctx.register_child(&mut self.label.p3); // ↖  ↑  ↗
        ctx.register_child(&mut self.label.p4);
        ctx.register_child(&mut self.label.p5);
        ctx.register_child(&mut self.label.p6); // ←  •  →
        ctx.register_child(&mut self.label.p7);
        ctx.register_child(&mut self.label.p8);
        ctx.register_child(&mut self.label.p9); // ↙  ↓  ↘
    }
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let min_height = theme::BORDERED_WIDGET_HEIGHT; // HACK: to make sure we look okay at default sizes when beside a textbox, we make sure we will have at least the same height as the default textbox.
        let mut lbl_pad9 = [
            (&mut self.label.p1, self.opt.pad.p1),
            (&mut self.label.p2, self.opt.pad.p2),
            (&mut self.label.p3, self.opt.pad.p3),
            (&mut self.label.p4, self.opt.pad.p4),
            (&mut self.label.p5, self.opt.pad.p5),
            (&mut self.label.p6, self.opt.pad.p6),
            (&mut self.label.p7, self.opt.pad.p7),
            (&mut self.label.p8, self.opt.pad.p8),
            (&mut self.label.p9, self.opt.pad.p9),
        ];

        let mut row_w: [f64; 3] = [0.; 3]; //top /middle/bottom ↖↑↗  ←•→  ↙↓↘
        let mut col_h: [f64; 3] = [0.; 3]; //left/center/right  ↖←↙  ↑•↓  ↗→↘
        let mut lsz: [Size; 10] = [Size::ZERO; 10];
        let mut lpad: [Insets; 10] = [Insets::ZERO; 10];
        for (i, (lbl9, pad9)) in lbl_pad9.iter_mut().enumerate() {
            let pad = match pad9 {
                Some(inset) => *inset,
                None => PAD_DEF,
            };
            let pad_sz = Size::new(pad.x_value(), pad.y_value());
            let lbl_bc = bc.shrink(pad_sz).loosen();
            let lbl_sz = ctx.run_layout(lbl9, &lbl_bc);
            if cfg!(debug_assertions) {
                let txt = ctx.get_raw_ref(lbl9).widget().text().clone();
                trace!(
                    "{} {} set layout l_sz = {} l_bc={:?} pad_sz={} txt={}",
                    i + 1,
                    ctx.widget_id(),
                    lbl_sz,
                    lbl_bc,
                    pad_sz,
                    txt
                );
            }
            if i == 4 {
                // set baseline off the central label only?
                let baseline = ctx.child_baseline_offset(&lbl9);
                ctx.set_baseline_offset(baseline + pad.y1);
            }
            if ROW_TOP.iter().any(|x| x == &i) {
                row_w[0] += lbl_sz.width + pad_sz.width;
            }
            if ROW_MID.iter().any(|x| x == &i) {
                row_w[1] += lbl_sz.width + pad_sz.width;
            }
            if ROW_BOT.iter().any(|x| x == &i) {
                row_w[2] += lbl_sz.width + pad_sz.width;
            }
            if COL_LHS.iter().any(|x| x == &i) {
                col_h[0] += lbl_sz.height + pad_sz.height;
            }
            if COL_CNT.iter().any(|x| x == &i) {
                col_h[1] += lbl_sz.height + pad_sz.height;
            }
            if COL_RHS.iter().any(|x| x == &i) {
                col_h[2] += lbl_sz.height + pad_sz.height;
            }
            lsz[i + 1] = lbl_sz; // store size for later offset calculations (after button size is known)
            lpad[i + 1] = pad;
        }
        let max_w = row_w[0].max(row_w[1]).max(row_w[2]);
        let max_h = col_h[0].max(col_h[1]).max(col_h[2]).max(min_height);
        let button_size = bc.constrain(Size::new(max_w, max_h));

        let bw = button_size.width;
        let bh = button_size.height; // ↖0,0 (w1-w2)/2=middle@x; (h1-h2)/2=center@y
        let lbl1_offset = Vec2::new(0. + lpad[1].x0, 0. + lpad[1].y0);
        let lbl2_offset = Vec2::new((bw - lsz[2].width) / 2.0, 0. + lpad[2].y0);
        let lbl3_offset = Vec2::new(bw - lsz[3].width - lpad[3].x1, 0. + lpad[3].y0);
        let lbl4_offset = Vec2::new(0. + lpad[4].x0, (bh - lsz[4].height) / 2.0);
        let lbl5_offset = (button_size.to_vec2() - lsz[5].to_vec2()) / 2.0;
        let lbl6_offset = Vec2::new(bw - lsz[6].width - lpad[6].x1, (bh - lsz[6].height) / 2.0);
        let lbl7_offset = Vec2::new(0. + lpad[7].x0, bh - lsz[7].height - lpad[7].y1);
        let lbl8_offset = Vec2::new((bw - lsz[8].width) / 2.0, bh - lsz[8].height - lpad[8].y1);
        let lbl9_offset = Vec2::new(
            bw - lsz[9].width - lpad[9].x1,
            bh - lsz[9].height - lpad[9].y1,
        ); // button_size.to_vec2() - lsz[3].to_vec2()
        if cfg!(debug_assertions) {
            trace!("button_size = {button_size:?} max_w={max_w} max_h={max_h} row_w={row_w:?} col_h={col_h:?}");
            trace!("↖ lbl1 🆔{} offset {}", ctx.widget_id(), lbl1_offset);
            trace!("↑ lbl2 🆔{} offset {}", ctx.widget_id(), lbl2_offset);
            trace!("↗ lbl3 🆔{} offset {}", ctx.widget_id(), lbl3_offset);
            trace!("← lbl4 🆔{} offset {}", ctx.widget_id(), lbl4_offset);
            trace!("• lbl5 🆔{} offset {}", ctx.widget_id(), lbl5_offset);
            trace!("→ lbl6 🆔{} offset {}", ctx.widget_id(), lbl6_offset);
            trace!("↙ lbl7 🆔{} offset {}", ctx.widget_id(), lbl7_offset);
            trace!("↓ lbl8 🆔{} offset {}", ctx.widget_id(), lbl8_offset);
            trace!("↘ lbl9 🆔{} offset {}", ctx.widget_id(), lbl9_offset);
        }

        ctx.place_child(&mut self.label.p1, lbl1_offset.to_point());
        ctx.place_child(&mut self.label.p2, lbl2_offset.to_point());
        ctx.place_child(&mut self.label.p3, lbl3_offset.to_point());
        ctx.place_child(&mut self.label.p4, lbl4_offset.to_point());
        ctx.place_child(&mut self.label.p5, lbl5_offset.to_point());
        ctx.place_child(&mut self.label.p6, lbl6_offset.to_point());
        ctx.place_child(&mut self.label.p7, lbl7_offset.to_point());
        ctx.place_child(&mut self.label.p8, lbl8_offset.to_point());
        ctx.place_child(&mut self.label.p9, lbl9_offset.to_point());
        button_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        let is_active = ctx.is_pointer_capture_target() && !ctx.is_disabled();
        let is_hovered = ctx.is_hovered();
        let size = ctx.size();
        let stroke_width = theme::BUTTON_BORDER_WIDTH;

        let rounded_rect = size
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(theme::BUTTON_BORDER_RADIUS);

        let bg_gradient = if ctx.is_disabled() {
            [theme::DISABLED_BUTTON_LIGHT, theme::DISABLED_BUTTON_DARK]
        } else if is_active {
            [theme::BUTTON_DARK, theme::BUTTON_LIGHT]
        } else {
            [theme::BUTTON_LIGHT, theme::BUTTON_DARK]
        };
        let border_color = if is_hovered && !ctx.is_disabled() {
            theme::BORDER_LIGHT
        } else {
            theme::BORDER_DARK
        };

        stroke(scene, &rounded_rect, border_color, stroke_width);
        fill_lin_gradient(
            scene,
            &rounded_rect,
            bg_gradient,
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }
    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut Node) {
        // IMPORTANT: We don't want to merge this code in practice, because the child label already has a 'name' property. This is more of a proof of concept of `get_raw_ref()`.
        if false {
            let label = ctx.get_raw_ref(&self.label.p5);
            let name = label.widget().text().as_ref().to_string();
            node.set_value(name);
        }
        node.add_action(accesskit::Action::Click);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        smallvec![
            self.label.p1.id(),
            self.label.p2.id(),
            self.label.p3.id(), // ↖  ↑  ↗
            self.label.p4.id(),
            self.label.p5.id(),
            self.label.p6.id(), // ←  •  →
            self.label.p7.id(),
            self.label.p8.id(),
            self.label.p9.id(), // ↙  ↓  ↘
        ]
    }
    fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {
        trace_span!("Button9", id = ctx.widget_id().trace())
    }
}

// --- MARK: TESTS ---
// #[cfg(test)] mod button9_test; //TODO

// TODO:
// how to adjust `accessibility` for all 9 labels?
// add tests
// reformat docs

// --- MARK: TESTS ---
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::assert_render_snapshot;
    use crate::core::StyleProperty;
    use crate::testing::{widget_ids, TestHarness, TestWidgetExt};
    use crate::theme::PRIMARY_LIGHT;

    #[test]
    fn simple_button() {
        let [button_id] = widget_ids();
        let widget = Button9::new("Hello").with_id(button_id);

        let mut harness = TestHarness::create(widget);

        assert_debug_snapshot!(harness.root_widget());
        assert_render_snapshot!(harness, "hello");

        assert_eq!(harness.pop_action(), None);

        harness.mouse_click_on(button_id);
        assert_eq!(
            harness.pop_action(),
            Some((Action::Button9Pressed(PointerButton9::Primary), button_id))
        );
    }

    #[test]
    fn edit_button() {
        let image_1 = {
            let label = Label::new("The quick brown fox jumps over the lazy dog")
                .with_brush(PRIMARY_LIGHT)
                .with_style(StyleProperty::FontSize(20.0));
            let button = Button9::from_label(label);

            let mut harness = TestHarness::create_with_size(button, Size::new(50.0, 50.0));

            harness.render()
        };

        let image_2 = {
            let button = Button9::new("Hello world");

            let mut harness = TestHarness::create_with_size(button, Size::new(50.0, 50.0));

            harness.edit_root_widget(|mut button| {
                let mut button = button.downcast::<Button9>();
                Button9::set_text(&mut button, "The quick brown fox jumps over the lazy dog");

                let mut label = Button9::label_mut(&mut button);
                Label::set_brush(&mut label, PRIMARY_LIGHT);
                Label::insert_style(&mut label, StyleProperty::FontSize(20.0));
            });

            harness.render()
        };

        // We don't use assert_eq because we don't want rich assert
        assert!(image_1 == image_2);
    }
}
