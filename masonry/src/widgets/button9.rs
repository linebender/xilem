// Copyright 2025 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A button widget with up to 9 labels.

use accesskit::{Node, Role};
use smallvec::{smallvec, SmallVec};
use tracing::{trace, debug, trace_span, Span};
use vello::Scene;

use masonry::core::{
  AccessCtx, AccessEvent, Action, ArcStr, BoxConstraints, EventCtx, LayoutCtx, PaintCtx,
  PointerButton, PointerEvent, QueryCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
  WidgetMut, WidgetPod,
};
use masonry::kurbo::{Insets, Size, Point, Vec2};
use masonry::theme;
use masonry::util::{fill_lin_gradient, stroke, UnitPoint};
use masonry::widgets::Label;

/// The minimum padding added to a button. NOTE: these values are chosen to match the existing look of TextBox; these should be reevaluated at some point.
pub const pad_def: Insets = Insets::uniform_xy(8., 2.);

/// IDs for all the 9 possible label positions in a button from top left to bottom right.<br>
  /// Letter corresponds to a visual mnemonic of its horizontal/vertical line position
  /// ⎺	T top   	⎸ L left
  /// -	H middle	| I middle
  /// _	L low   	⎹ J right
  /// ⎺T -H _L  top/middle/low
  /// ↖  ↑  ↗
  /// ←  •  →
  /// ↙  ↓  ↘
pub enum LPos {
  TL1 = 1, TI2 = 2, TJ3 = 3,
  HL4 = 4, HI5 = 5, HJ6 = 6,
  LL7 = 7, LI8 = 8, LJ9 = 9,
}
/// A button with up to 9 text Labels (allowing for custom styles) with custom padding
/// (allowing for flexible positioning).
pub struct Button9 {
  /// 9 label widgets
  label: Label9  ,
  /// Options for those widgets or the button as a whole (only padding is implemented)
  opt  : LabelOpt,
}
/// Label widgets for Button9
pub struct Label9 {
  TL1:WidgetPod<Label>, TI2:WidgetPod<Label>, TJ3:WidgetPod<Label>, // ↖  ↑  ↗
  HL4:WidgetPod<Label>, HI5:WidgetPod<Label>, HJ6:WidgetPod<Label>, // ←  •  →
  LL7:WidgetPod<Label>, LI8:WidgetPod<Label>, LJ9:WidgetPod<Label>, // ↙  ↓  ↘
}
/// Custom button options. Currently only padding is supported.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct LabelOpt {
  /// Per-label padding.
  pub pad: Pad9,
  // pub is : Is9, //
}

/// Optional padding options per label as [`Insets`]
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Pad9 {
  pub TL1:Option<Insets>, pub TI2:Option<Insets>, pub TJ3:Option<Insets>, // ↖  ↑  ↗
  pub HL4:Option<Insets>, pub HI5:Option<Insets>, pub HJ6:Option<Insets>, // ←  •  →
  pub LL7:Option<Insets>, pub LI8:Option<Insets>, pub LJ9:Option<Insets>, // ↙  ↓  ↘
}
// /// Track whether a label exists, useful for layout constraint calculations
// #[derive(Default, Debug, Copy, Clone, PartialEq)]
// pub struct Is9 {
//   pub TL1:bool, pub TI2:bool, pub TJ3:bool, // ↖  ↑  ↗
//   pub HL4:bool, pub HI5:bool, pub HJ6:bool, // ←  •  →
//   pub LL7:bool, pub LI8:bool, pub LJ9:bool, // ↙  ↓  ↘
// }

// --- MARK: BUILDERS ---
impl Button9 {
  /// Create a new button with a text label at the center (HI5m other labels are blank, use `.addx` methods to fill them)
  /// ```
  /// use masonry::widgets::Button9;
  /// let button = Button9::new("Increment");
  /// ```
  pub fn new(text:impl Into<ArcStr>) -> Self {Self::from_label    (Label::new(text))}
  /// Create a new button with the provided [`Label`]
  /// ```
  /// use masonry::peniko::Color;
  /// use masonry::widgets::{Button9, Label};
  /// let label = Label::new("Increment").with_brush(Color::new([0.5, 0.5, 0.5, 1.0]));
  /// let button = Button9::from_label(label);
  /// ```
  pub fn from_label    (label:Label) -> Self {Self::from_label_pad(label, None)}
  /// Create a new button with the provided [`Label`] and padding [`Insets`]
  /// ```
  /// use masonry::peniko::Color;
  /// use masonry::widgets::{Button9, Label};
  /// let label  = Label::new("Increment").with_brush(Color::new([0.5, 0.5, 0.5, 1.0]));
  /// let pad    = Insets::uniform_xy(8., 2.); // pad ←→ by 8 and ↑↓ by 2
  /// let button = Button9::from_label_pad(label, pad);
  /// ```
  pub fn from_label_pad(lbl:Label, pad:Option<Insets>) -> Self {
    // let is = is9 {
      // TL1:false, TI2:false                , TJ3:false, // ↖  ↑  ↗
      // HL4:false, HI5:lbl.text().len() > 0 , HJ6:false, // ←  •  →
      // LL7:false, LI8:false                , LJ9:false, // ↙  ↓  ↘
    // };
    let label = Label9 {
      TL1:WidgetPod::new(Label::new("")), TI2:WidgetPod::new(Label::new("")), TJ3:WidgetPod::new(Label::new("")), // ↖  ↑  ↗
      HL4:WidgetPod::new(Label::new("")), HI5:WidgetPod::new(lbl           ), HJ6:WidgetPod::new(Label::new("")), // ←  •  →
      LL7:WidgetPod::new(Label::new("")), LI8:WidgetPod::new(Label::new("")), LJ9:WidgetPod::new(Label::new("")), // ↙  ↓  ↘
    };
    let pad = Pad9 {
      TL1:None, TI2:None, TJ3:None, // ↖  ↑  ↗
      HL4:None, HI5:pad , HJ6:None, // ←  •  →
      LL7:None, LI8:None, LJ9:None, // ↙  ↓  ↘
    };
    let opt = LabelOpt{pad}; //, is
    Self {label, opt}
  }
  /// Helper .methods for adding individual labels (add=center HI5)
  pub fn add (mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.HI5 = WidgetPod::new(label); self.opt.pad.HI5 = pad; self}
  pub fn add1(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.TL1 = WidgetPod::new(label); self.opt.pad.TL1 = pad; self}
  pub fn add2(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.TI2 = WidgetPod::new(label); self.opt.pad.TI2 = pad; self}
  pub fn add3(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.TJ3 = WidgetPod::new(label); self.opt.pad.TJ3 = pad; self}
  pub fn add4(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.HL4 = WidgetPod::new(label); self.opt.pad.HL4 = pad; self}
  pub fn add5(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.HI5 = WidgetPod::new(label); self.opt.pad.HI5 = pad; self}
  pub fn add6(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.HJ6 = WidgetPod::new(label); self.opt.pad.HJ6 = pad; self}
  pub fn add7(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.LL7 = WidgetPod::new(label); self.opt.pad.LL7 = pad; self}
  pub fn add8(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.LI8 = WidgetPod::new(label); self.opt.pad.LI8 = pad; self}
  pub fn add9(mut self,         label:Label, pad:Option<Insets>) -> Self {self.label.LJ9 = WidgetPod::new(label); self.opt.pad.LJ9 = pad; self}
  // pub fn add (mut self,         label:Label, pad:Option<Insets>) {self.addx(LPos::HI5,label,pad)}
  /// Helper .method for adding a label to a given position (same as in [`LPos`])
  pub fn addx(mut self,idx:LPos,label:Label, pad:Option<Insets>) -> Self {match idx {
    LPos::TL1 => {self.label.TL1 = WidgetPod::new(label); self.opt.pad.TL1 = pad}, //↖
    LPos::TI2 => {self.label.TI2 = WidgetPod::new(label); self.opt.pad.TI2 = pad}, //↑
    LPos::TJ3 => {self.label.TJ3 = WidgetPod::new(label); self.opt.pad.TJ3 = pad}, //↗
    LPos::HL4 => {self.label.HL4 = WidgetPod::new(label); self.opt.pad.HL4 = pad}, //←
    LPos::HI5 => {self.label.HI5 = WidgetPod::new(label); self.opt.pad.HI5 = pad}, //•
    LPos::HJ6 => {self.label.HJ6 = WidgetPod::new(label); self.opt.pad.HJ6 = pad}, //→
    LPos::LL7 => {self.label.LL7 = WidgetPod::new(label); self.opt.pad.LL7 = pad}, //↙
    LPos::LI8 => {self.label.LI8 = WidgetPod::new(label); self.opt.pad.LI8 = pad}, //↓
    LPos::LJ9 => {self.label.LJ9 = WidgetPod::new(label); self.opt.pad.LJ9 = pad}, //↘
  } self }
  /// Create a new button with the provided [`Label9`]s and their [`Pad9`] with predetermined IDs. This constructor is useful for toolkits which use Masonry (such as Xilem).
  pub fn from_label_pod(label_l:[WidgetPod<Label>;9], pad:Pad9) -> Self { //, is:is9
    let [l1,l2,l3,l4,l5,l6,l7,l8,l9] = label_l;
    let label = Label9 { //numbering shifted due to 0-based array index
      TL1:l1, TI2:l2, TJ3:l3, // ↖  ↑  ↗
      HL4:l4, HI5:l5, HJ6:l8, // ←  •  →
      LL7:l7, LI8:l6, LJ9:l9, // ↙  ↓  ↘
    };
    let opt = LabelOpt{pad}; //, is
    Self {label, opt}
  }
}

// Helper indices for the Label9 positions (0-based unlike .Prop or fn() names!)
const row_top: [usize;3] = [0,1,2]; //↖ ↑ ↗
const row_mid: [usize;3] = [3,4,5]; //← • →
const row_bot: [usize;3] = [6,7,8]; //↙ ↓ ↘
const col_lhs: [usize;3] = [0,1,2]; //↖ ← ↙
const col_cnt: [usize;3] = [3,4,5]; //↑ • ↓
const col_rhs: [usize;3] = [6,7,8]; //↗ → ↘

// --- MARK: WIDGETMUT ---
impl Button9 {
  /// Set text helpers
  pub fn set_text (this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label_mut (this), new_text);}
  pub fn set_text1(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label1_mut(this), new_text);}
  pub fn set_text2(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label2_mut(this), new_text);}
  pub fn set_text3(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label3_mut(this), new_text);}
  pub fn set_text4(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label4_mut(this), new_text);}
  pub fn set_text5(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label5_mut(this), new_text);}
  pub fn set_text6(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label6_mut(this), new_text);}
  pub fn set_text7(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label7_mut(this), new_text);}
  pub fn set_text8(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label8_mut(this), new_text);}
  pub fn set_text9(this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label9_mut(this), new_text);}
  // pub fn set_text (this:&mut WidgetMut<'_,Self>, new_text:impl Into<ArcStr>) {Label::set_text(&mut Self::label_mutx(this,LPos::HI5), new_text);}
  /// Set label text for a given position
  pub fn set_textx(this:&mut WidgetMut<'_,Self>, idx:LPos, new_text: impl Into<ArcStr>) {
    Label::set_text(&mut Self::labelx_mut(this, idx), new_text);
  }

  /// Set label options helpers
  pub fn set_opt <'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.HI5 = new_pad; this.ctx.request_render();}
  pub fn set_pad1<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.TL1 = new_pad; this.ctx.request_render();}
  pub fn set_pad2<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.TI2 = new_pad; this.ctx.request_render();}
  pub fn set_pad3<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.TJ3 = new_pad; this.ctx.request_render();}
  pub fn set_pad4<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.HL4 = new_pad; this.ctx.request_render();}
  pub fn set_pad5<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.HI5 = new_pad; this.ctx.request_render();}
  pub fn set_pad6<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.HJ6 = new_pad; this.ctx.request_render();}
  pub fn set_pad7<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.LL7 = new_pad; this.ctx.request_render();}
  pub fn set_pad8<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.LI8 = new_pad; this.ctx.request_render();}
  pub fn set_pad9<'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.widget.opt.pad.LJ9 = new_pad; this.ctx.request_render();}
  // pub fn set_opt  <'t>(this: &'t mut WidgetMut<'_,Self>, new_pad:Option<Insets>) {this.set_optx(LPos::HI5, new_opt);}
  /// Set the label options for a given position
  pub fn set_padx(this: &mut WidgetMut<'_,Self>, idx:LPos, new_pad:Option<Insets>) {match idx {
    LPos::TL1 => {this.widget.opt.pad.TL1 = new_pad}, //↖
    LPos::TI2 => {this.widget.opt.pad.TI2 = new_pad}, //↑
    LPos::TJ3 => {this.widget.opt.pad.TJ3 = new_pad}, //↗
    LPos::HL4 => {this.widget.opt.pad.HL4 = new_pad}, //←
    LPos::HI5 => {this.widget.opt.pad.HI5 = new_pad}, //•
    LPos::HJ6 => {this.widget.opt.pad.HJ6 = new_pad}, //→
    LPos::LL7 => {this.widget.opt.pad.LL7 = new_pad}, //↙
    LPos::LI8 => {this.widget.opt.pad.LI8 = new_pad}, //↓
    LPos::LJ9 => {this.widget.opt.pad.LJ9 = new_pad}, //↘
    }
    this.ctx.request_render(); // label options state impacts appearance and accessibility node
  }

  /// Get mutable label helpers
  pub fn label_mut <'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.HI5)}
  pub fn label1_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.TL1)}
  pub fn label2_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.TI2)}
  pub fn label3_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.TJ3)}
  pub fn label4_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.HL4)}
  pub fn label5_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.HI5)}
  pub fn label6_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.HJ6)}
  pub fn label7_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.LL7)}
  pub fn label8_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.LI8)}
  pub fn label9_mut<'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.ctx.get_mut(&mut this.widget.label.LJ9)}
  // pub fn label_mut <'t>(this: &'t mut WidgetMut<'_,Self>) -> WidgetMut<'t, Label> {this.labelx_mut(LPos::HI5)}
  /// Get mutable label for a given position
  pub fn labelx_mut<'t>(this: &'t mut WidgetMut<'_,Self>, idx:LPos) -> WidgetMut<'t, Label> {match idx {
    LPos::TL1 => {return this.ctx.get_mut(&mut this.widget.label.TL1)}, //↖
    LPos::TI2 => {return this.ctx.get_mut(&mut this.widget.label.TI2)}, //↑
    LPos::TJ3 => {return this.ctx.get_mut(&mut this.widget.label.TJ3)}, //↗
    LPos::HL4 => {return this.ctx.get_mut(&mut this.widget.label.HL4)}, //←
    LPos::HI5 => {return this.ctx.get_mut(&mut this.widget.label.HI5)}, //•
    LPos::HJ6 => {return this.ctx.get_mut(&mut this.widget.label.HJ6)}, //→
    LPos::LL7 => {return this.ctx.get_mut(&mut this.widget.label.LL7)}, //↙
    LPos::LI8 => {return this.ctx.get_mut(&mut this.widget.label.LI8)}, //↓
    LPos::LJ9 => {return this.ctx.get_mut(&mut this.widget.label.LJ9)}, //↘
  }}
}

// --- MARK: IMPL WIDGET ---
impl Widget for Button9 {
  fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
    match event {
      PointerEvent::PointerDown(_, _) => {
        if !ctx.is_disabled() {ctx.capture_pointer(); ctx.request_paint_only(); // Changes in pointer capture impact appearance, but not accessibility node
          trace!("Button9 {:?} pressed", ctx.widget_id());}
      }
      PointerEvent::PointerUp(button, _) => {
        if ctx.is_pointer_capture_target() && ctx.is_hovered() && !ctx.is_disabled() {
          ctx.submit_action(Action::ButtonPressed(*button));
          trace!("Button9 {:?} released", ctx.widget_id());}
        ctx.request_paint_only(); // Changes in pointer capture impact appearance, but not accessibility node
      }
      _ => (),
    }
  }

  fn on_text_event  (&mut self, _ctx: &mut EventCtx, _event: &TextEvent) {}

  fn on_access_event(&mut self,  ctx: &mut EventCtx,  event: &AccessEvent) {
    if ctx.target() == ctx.widget_id() { match event.action {
      accesskit::Action::Click  => {ctx.submit_action(Action::ButtonPressed(PointerButton::Primary));}
      _                         => {} }  }   }

  fn update(&mut self, ctx: &mut UpdateCtx, event: &Update) { match event {
     Update::HoveredChanged (_)
    |Update::FocusChanged   (_)
    |Update::DisabledChanged(_) => {ctx.request_paint_only();}
    _                           => {}  }   }

  fn register_children(&mut self, ctx: &mut masonry::core::RegisterCtx) {
    ctx.register_child(&mut self.label.TL1);ctx.register_child(&mut self.label.TI2);ctx.register_child(&mut self.label.TJ3); // ↖  ↑  ↗
    ctx.register_child(&mut self.label.HL4);ctx.register_child(&mut self.label.HI5);ctx.register_child(&mut self.label.HJ6); // ←  •  →
    ctx.register_child(&mut self.label.LL7);ctx.register_child(&mut self.label.LI8);ctx.register_child(&mut self.label.LJ9); // ↙  ↓  ↘
  }
  fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
    let min_height = theme::BORDERED_WIDGET_HEIGHT; // HACK: to make sure we look okay at default sizes when beside a textbox, we make sure we will have at least the same height as the default textbox.
    let mut lbl_pad9 = [
      (&mut self.label.TL1, self.opt.pad.TL1),
      (&mut self.label.TI2, self.opt.pad.TI2),
      (&mut self.label.TJ3, self.opt.pad.TJ3),
      (&mut self.label.HL4, self.opt.pad.HL4),
      (&mut self.label.HI5, self.opt.pad.HI5),
      (&mut self.label.HJ6, self.opt.pad.HJ6),
      (&mut self.label.LL7, self.opt.pad.LL7),
      (&mut self.label.LI8, self.opt.pad.LI8),
      (&mut self.label.LJ9, self.opt.pad.LJ9),
      ];

    let mut row_w: [f64 ;3] = [0.   ;3]; //top /middle/bottom ↖↑↗  ←•→  ↙↓↘
    let mut col_h: [f64 ;3] = [0.   ;3]; //left/center/right  ↖←↙  ↑•↓  ↗→↘
    // let mut is   : [bool;9] = [false;9];
    let mut lsz  : [Size  ;10] = [Size::ZERO  ;10];
    let mut lpad : [Insets;10] = [Insets::ZERO;10];
    // let mut valid = vec!();
    for (i, (lbl9, pad9)) in lbl_pad9.iter_mut().enumerate() {
      // let is_txt = ctx.get_raw_ref(lbl9).widget().text().len() > 0;
      // is[i] = is_txt;
      // if is_txt || i == 4 { //todo only calc/place valid labels,
        // but then update register_children + children_ids
        // and call children_changed when set_txt sets what was an empty label
      let pad       = match pad9 {Some(inset)=>*inset, None=>pad_def,};
      let pad_sz    = Size::new(pad.x_value(), pad.y_value());
      let lbl_bc    = bc.shrink(pad_sz).loosen();
      let lbl_sz    = ctx.run_layout(lbl9, &lbl_bc);
      if cfg!(debug_assertions) {let txt = ctx.get_raw_ref(lbl9).widget().text().clone();
        trace!("{} {} set layout l_sz = {} l_bc={:?} pad_sz={} txt={}", i+1, ctx.widget_id(),lbl_sz,lbl_bc,pad_sz,txt);}
      if i == 4 { // set baseline off the central label only?
        let baseline = ctx.child_baseline_offset(&lbl9);
        ctx.set_baseline_offset(baseline + pad.y1);
      }
      if row_top.iter().any(|x| x == &i) {row_w[0] += lbl_sz.width  + pad_sz.width ;}
      if row_mid.iter().any(|x| x == &i) {row_w[1] += lbl_sz.width  + pad_sz.width ;}
      if row_bot.iter().any(|x| x == &i) {row_w[2] += lbl_sz.width  + pad_sz.width ;}
      if col_lhs.iter().any(|x| x == &i) {col_h[0] += lbl_sz.height + pad_sz.height;}
      if col_cnt.iter().any(|x| x == &i) {col_h[1] += lbl_sz.height + pad_sz.height;}
      if col_rhs.iter().any(|x| x == &i) {col_h[2] += lbl_sz.height + pad_sz.height;}
      lsz[i+1] = lbl_sz; // store size for later offset calculations (after button size is known)
      lpad[i+1] = pad;
      // valid.push((i, lbl9));
      // }
    }
    let max_w = row_w[0].max(row_w[1]).max(row_w[2]);
    let max_h = col_h[0].max(col_h[1]).max(col_h[2]).max(min_height);
    let button_size = bc.constrain(Size::new(max_w, max_h));

    // for (i, lbl9) in valid { // for (i, (lbl9, pad9)) in lbl_pad9.iter_mut().enumerate() {
    let bw = button_size.width; let bh = button_size.height; // ↖0,0 (w1-w2)/2=middle@x; (h1-h2)/2=center@y
    let lbl1_offset = Vec2::new( 0.                     + lpad[1].x0    , 0.                      + lpad[1].y0  );
    let lbl2_offset = Vec2::new((bw - lsz[2].width)/2.0                 , 0.                      + lpad[2].y0  );
    let lbl3_offset = Vec2::new( bw - lsz[3].width      - lpad[3].x1    , 0.                      + lpad[3].y0  );
    let lbl4_offset = Vec2::new( 0.                     + lpad[4].x0    ,(bh - lsz[4].height)/2.0               );
    let lbl5_offset =(button_size.to_vec2() - lsz[5].to_vec2())/2.0                                              ;
    let lbl6_offset = Vec2::new( bw - lsz[6].width      - lpad[6].x1    ,(bh - lsz[6].height)/2.0               );
    let lbl7_offset = Vec2::new( 0.                     + lpad[7].x0    , bh - lsz[7].height     - lpad[7].y1   );
    let lbl8_offset = Vec2::new((bw - lsz[8].width)/2.0                 , bh - lsz[8].height     - lpad[8].y1   );
    let lbl9_offset = Vec2::new( bw - lsz[9].width      - lpad[9].x1    , bh - lsz[9].height     - lpad[9].y1   ); // button_size.to_vec2() - lsz[3].to_vec2()
    if cfg!(debug_assertions) {
      trace!("button_size = {button_size:?} max_w={max_w} max_h={max_h} row_w={row_w:?} col_h={col_h:?}");
      trace!("↖ lbl1 🆔{} offset {}", ctx.widget_id(), lbl1_offset); trace!("↑ lbl2 🆔{} offset {}", ctx.widget_id(), lbl2_offset); trace!("↗ lbl3 🆔{} offset {}", ctx.widget_id(), lbl3_offset);
      trace!("← lbl4 🆔{} offset {}", ctx.widget_id(), lbl4_offset); trace!("• lbl5 🆔{} offset {}", ctx.widget_id(), lbl5_offset); trace!("→ lbl6 🆔{} offset {}", ctx.widget_id(), lbl6_offset);
      trace!("↙ lbl7 🆔{} offset {}", ctx.widget_id(), lbl7_offset); trace!("↓ lbl8 🆔{} offset {}", ctx.widget_id(), lbl8_offset); trace!("↘ lbl9 🆔{} offset {}", ctx.widget_id(), lbl9_offset);
    }

    ctx.place_child(&mut self.label.TL1, lbl1_offset.to_point()); ctx.place_child(&mut self.label.TI2, lbl2_offset.to_point()); ctx.place_child(&mut self.label.TJ3, lbl3_offset.to_point());
    ctx.place_child(&mut self.label.HL4, lbl4_offset.to_point()); ctx.place_child(&mut self.label.HI5, lbl5_offset.to_point()); ctx.place_child(&mut self.label.HJ6, lbl6_offset.to_point());
    ctx.place_child(&mut self.label.LL7, lbl7_offset.to_point()); ctx.place_child(&mut self.label.LI8, lbl8_offset.to_point()); ctx.place_child(&mut self.label.LJ9, lbl9_offset.to_point());
    // }
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

    let bg_gradient = if ctx.is_disabled()                  {[theme::DISABLED_BUTTON_LIGHT, theme::DISABLED_BUTTON_DARK]
    } else if is_active                                     {[theme::BUTTON_DARK          , theme::BUTTON_LIGHT]
    } else                                                  {[theme::BUTTON_LIGHT         , theme::BUTTON_DARK]};
    let border_color = if is_hovered && !ctx.is_disabled()  { theme::BORDER_LIGHT
    } else                                                  { theme::BORDER_DARK};

    stroke           (scene, &rounded_rect, border_color, stroke_width);
    fill_lin_gradient(scene, &rounded_rect, bg_gradient, UnitPoint::TOP,UnitPoint::BOTTOM,);
  }

  fn accessibility_role(&self) -> Role {Role::Button}
  fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut Node) { // IMPORTANT: We don't want to merge this code in practice, because the child label already has a 'name' property. This is more of a proof of concept of `get_raw_ref()`.
    if false {
      let label = ctx.get_raw_ref(&self.label.HI5);
      let name  = label.widget().text().as_ref().to_string();
      node.set_value(name);
    }
    node.add_action(accesskit::Action::Click);
  }

  fn children_ids   (&self                    ) -> SmallVec<[WidgetId; 16]> {smallvec![
    self.label.TL1.id(),self.label.TI2.id(),self.label.TJ3.id(), // ↖  ↑  ↗
    self.label.HL4.id(),self.label.HI5.id(),self.label.HJ6.id(), // ←  •  →
    self.label.LL7.id(),self.label.LI8.id(),self.label.LJ9.id(), // ↙  ↓  ↘
  ]}
  fn make_trace_span(&self, ctx: &QueryCtx<'_>) -> Span {trace_span!("Button9", id = ctx.widget_id().trace())}
}

// --- MARK: TESTS ---
// #[cfg(test)] mod button9_test; //TODO

// TODO:
  // how to adjust `accessibility` for all 9 labels?
  // add tests
  // reformat docs
