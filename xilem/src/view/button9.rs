// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

pub use masonry::core::{PointerButton, PointerButton as PointerB, WidgetPod};
use masonry::widgets::{self, button9 as widget9};

use crate::core::{ViewPathTracker,
  DynMessage, Mut, View, ViewMarker,
  MessageResult, ViewId, MessageResult as MsgRes,
};
use crate::view::Label;
use crate::{Pod, ViewCtx};

use masonry::kurbo::Insets;

/// A button which calls `callback` when the 🖰1 (normally left) is pressed<br>
/// To use button provide it with a button text or `label` and a closure
/// ```ignore
/// use xilem::view::{button, label};
/// struct State { int: i32 }
/// impl   State { fn increase(&mut self) { self.int += 1 ;} }
/// let   label =       "Button"; // or ↓
/// //let label = label("Button").weight(FontWeight::BOLD);
/// button(label, |state:&mut State| { state.increase();})
/// ```
pub fn button9<State,Action> (label:impl Into<Label>, callback:impl Fn(&mut State) -> Action+Send+'static)
->ButtonL9<impl for<'a> Fn(&'a mut State, PointerB) -> MsgRes<Action>+Send+'static> {
  button9_pad(label, None, callback)
}
/// A button with custom `pad` padding which calls `callback` when 🖰1 (normally left) is pressed
pub fn button9_pad<State,Action>(label:impl Into<Label>, pad:Option<Insets>, callback: impl Fn(&mut State) -> Action+Send+'static)
->ButtonL9<impl for<'a> Fn(&'a mut State, PointerB) -> MsgRes<Action>+Send+'static> {
  ButtonL9::new(label, pad,
    move |state: &mut State, button| match button {
      PointerB::Primary => MsgRes::Action(callback(state)),
      _                 => MsgRes::Nop                    ,},
  )
}
/// A button which calls `callback` when any 🖰 is pressed
pub fn button9_any_pointer<State,Action>(label:impl Into<Label>, callback: impl Fn(&mut State, PointerB) -> Action+Send+'static)
->ButtonL9<impl for<'a> Fn(&'a mut State, PointerB) -> MsgRes<Action>+Send+'static> {
  button9_any_pointer_pad(label, None, callback)
}
/// A button with custom `pad` padding which calls `callback` when any 🖰 is pressed
pub fn button9_any_pointer_pad<State,Action>(label:impl Into<Label>, pad:Option<Insets>, callback: impl Fn(&mut State, PointerB) -> Action+Send+'static)
->ButtonL9<impl for<'a> Fn(&'a mut State, PointerB) -> MsgRes<Action>+Send+'static> {
  ButtonL9::new(label, pad,
    move |state: &mut State, button| MsgRes::Action(callback(state, button)),
  )
}

use crate::masonry::button9::{LPos, LabelOpt, Pad9};

/// The [`View`] created by [`button`] from up to label(s) in one of [`LPos`] position with custom padding and a callback.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ButtonL9<F> {
  label: Label9,
  opt  : LabelOpt,
  callback: F,
}
/// Label for ButtonL9
pub struct Label9 {
  tl1:Label, ti2:Label, tj3:Label, // ↖  ↑  ↗
  hl4:Label, hi5:Label, hj6:Label, // ←  •  →
  ll7:Label, li8:Label, lj9:Label, // ↙  ↓  ↘
}

impl<F> ButtonL9<F>{
  /// Create a new button with a text label at the center (hi5m other labels are blank, use `.addx` methods to fill them)
  pub fn new(                     label:impl Into<Label>, pad:Option<Insets>, callback:F) -> Self {
    let label = Label9 {
      tl1:"".into(), ti2:""   .into(), tj3:"".into(), // ↖  ↑  ↗
      hl4:"".into(), hi5:label.into(), hj6:"".into(), // ←  •  →
      ll7:"".into(), li8:""   .into(), lj9:"".into(), // ↙  ↓  ↘
    };
    let pad = Pad9 {
      tl1:None, ti2:None, tj3:None, // ↖  ↑  ↗
      hl4:None, hi5:pad , hj6:None, // ←  •  →
      ll7:None, li8:None, lj9:None, // ↙  ↓  ↘
    };
    let opt = LabelOpt{pad};
    Self {label, opt, callback}
  }
  /// Helper .methods for adding individual labels (add=center hi5)
  pub fn add (mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.hi5 = label.into(); self.opt.pad.hi5 = pad; self}
  pub fn add1(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.tl1 = label.into(); self.opt.pad.tl1 = pad; self}
  pub fn add2(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.ti2 = label.into(); self.opt.pad.ti2 = pad; self}
  pub fn add3(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.tj3 = label.into(); self.opt.pad.tj3 = pad; self}
  pub fn add4(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.hl4 = label.into(); self.opt.pad.hl4 = pad; self}
  pub fn add5(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.hi5 = label.into(); self.opt.pad.hi5 = pad; self}
  pub fn add6(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.hj6 = label.into(); self.opt.pad.hj6 = pad; self}
  pub fn add7(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.ll7 = label.into(); self.opt.pad.ll7 = pad; self}
  pub fn add8(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.li8 = label.into(); self.opt.pad.li8 = pad; self}
  pub fn add9(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.lj9 = label.into(); self.opt.pad.lj9 = pad; self}
  // pub fn add (mut self,         label:impl Into<Label>, pad:Option<Insets>) {self.addx(LPos::hi5,label,pad)}
  /// Helper .method for adding a label to a given position (same as in [`LPos`])
  pub fn addx(mut self,idx:LPos,label:impl Into<Label>, pad:Option<Insets>) -> Self {match idx {
    LPos::tl1 => {self.label.tl1 = label.into(); self.opt.pad.tl1 = pad}, //↖
    LPos::ti2 => {self.label.ti2 = label.into(); self.opt.pad.ti2 = pad}, //↑
    LPos::tj3 => {self.label.tj3 = label.into(); self.opt.pad.tj3 = pad}, //↗
    LPos::hl4 => {self.label.hl4 = label.into(); self.opt.pad.hl4 = pad}, //←
    LPos::hi5 => {self.label.hi5 = label.into(); self.opt.pad.hi5 = pad}, //•
    LPos::hj6 => {self.label.hj6 = label.into(); self.opt.pad.hj6 = pad}, //→
    LPos::ll7 => {self.label.ll7 = label.into(); self.opt.pad.ll7 = pad}, //↙
    LPos::li8 => {self.label.li8 = label.into(); self.opt.pad.li8 = pad}, //↓
    LPos::lj9 => {self.label.lj9 = label.into(); self.opt.pad.lj9 = pad}, //↘
  } self }
}

const id_lvw1: ViewId = ViewId::new(1);
const id_lvw2: ViewId = ViewId::new(2);
const id_lvw3: ViewId = ViewId::new(3);
const id_lvw4: ViewId = ViewId::new(4);
const id_lvw5: ViewId = ViewId::new(5);
const id_lvw6: ViewId = ViewId::new(6);
const id_lvw7: ViewId = ViewId::new(7);
const id_lvw8: ViewId = ViewId::new(8);
const id_lvw9: ViewId = ViewId::new(9);

pub fn into_widget_pod(p:Pod<widgets::Label>) -> WidgetPod<widgets::Label> {
  WidgetPod::new_with_id_and_transform(p.widget, p.id, p.transform)
}

impl<F>              ViewMarker                 for ButtonL9<F> {}
impl<F,State,Action> View<State,Action,ViewCtx> for ButtonL9<F>
where F: Fn(&mut State, PointerB) -> MsgRes<Action> + Send + Sync + 'static {
  type Element = Pod<widget9::ButtonL9>;
  type ViewState = ();

  fn build(&self, ctx:&mut ViewCtx) -> (Self::Element, Self::ViewState) {
    // build based on LabelViews, which already implement build themselves:(Self::Element, Self::ViewState)
    let (child1,()) = ctx.with_id(id_lvw1, |ctx|{View::<State,Action,_>::build(&self.label.tl1,ctx)});
    let (child2,()) = ctx.with_id(id_lvw2, |ctx|{View::<State,Action,_>::build(&self.label.ti2,ctx)});
    let (child3,()) = ctx.with_id(id_lvw3, |ctx|{View::<State,Action,_>::build(&self.label.tj3,ctx)});
    let (child4,()) = ctx.with_id(id_lvw4, |ctx|{View::<State,Action,_>::build(&self.label.hl4,ctx)});
    let (child5,()) = ctx.with_id(id_lvw5, |ctx|{View::<State,Action,_>::build(&self.label.hi5,ctx)});
    let (child6,()) = ctx.with_id(id_lvw6, |ctx|{View::<State,Action,_>::build(&self.label.hj6,ctx)});
    let (child7,()) = ctx.with_id(id_lvw7, |ctx|{View::<State,Action,_>::build(&self.label.ll7,ctx)});
    let (child8,()) = ctx.with_id(id_lvw8, |ctx|{View::<State,Action,_>::build(&self.label.li8,ctx)});
    let (child9,()) = ctx.with_id(id_lvw9, |ctx|{View::<State,Action,_>::build(&self.label.lj9,ctx)});
    // pass built elements to the masonry widgets
    let label = [
      into_widget_pod(child1), into_widget_pod(child2), into_widget_pod(child3), // ↖  ↑  ↗
      into_widget_pod(child4), into_widget_pod(child5), into_widget_pod(child8), // ←  •  →
      into_widget_pod(child7), into_widget_pod(child6), into_widget_pod(child9), // ↙  ↓  ↘
      // child1.into_widget_pod(), child2.into_widget_pod(), child3.into_widget_pod(), // ↖  ↑  ↗
      // child4.into_widget_pod(), child5.into_widget_pod(), child8.into_widget_pod(), // ←  •  →
      // child7.into_widget_pod(), child6.into_widget_pod(), child9.into_widget_pod(), // ↙  ↓  ↘
    ];
    let pad = Pad9 {
      tl1:self.opt.pad.tl1.clone(), ti2:self.opt.pad.ti2.clone(), tj3:self.opt.pad.tj3.clone(), // ↖  ↑  ↗
      hl4:self.opt.pad.hl4.clone(), hi5:self.opt.pad.hi5.clone(), hj6:self.opt.pad.hj6.clone(), // ←  •  →
      ll7:self.opt.pad.ll7.clone(), li8:self.opt.pad.li8.clone(), lj9:self.opt.pad.lj9.clone(), // ↙  ↓  ↘
    };
    ctx.with_leaf_action_widget(|ctx| {ctx.new_pod(widget9::ButtonL9::from_label_pod(label,pad)) } )
  }

  fn rebuild(&self, prev:&Self, state:&mut Self::ViewState, ctx:&mut ViewCtx, mut el:Mut<Self::Element>) {
    // rebuild based on LabelViews, which already implement rebuild themselves (compare all the props)
    ctx.with_id(id_lvw1, |ctx|{View::<State,Action,_>::rebuild(&self.label.tl1,&prev.label.tl1,state,ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw2, |ctx|{View::<State,Action,_>::rebuild(&self.label.ti2,&prev.label.ti2,state,ctx, widget9::ButtonL9::label2_mut(&mut el));});
    ctx.with_id(id_lvw3, |ctx|{View::<State,Action,_>::rebuild(&self.label.tj3,&prev.label.tj3,state,ctx, widget9::ButtonL9::label3_mut(&mut el));});
    ctx.with_id(id_lvw4, |ctx|{View::<State,Action,_>::rebuild(&self.label.hl4,&prev.label.hl4,state,ctx, widget9::ButtonL9::label4_mut(&mut el));});
    ctx.with_id(id_lvw5, |ctx|{View::<State,Action,_>::rebuild(&self.label.hi5,&prev.label.hi5,state,ctx, widget9::ButtonL9::label5_mut(&mut el));});
    ctx.with_id(id_lvw6, |ctx|{View::<State,Action,_>::rebuild(&self.label.hj6,&prev.label.hj6,state,ctx, widget9::ButtonL9::label6_mut(&mut el));});
    ctx.with_id(id_lvw7, |ctx|{View::<State,Action,_>::rebuild(&self.label.ll7,&prev.label.ll7,state,ctx, widget9::ButtonL9::label7_mut(&mut el));});
    ctx.with_id(id_lvw8, |ctx|{View::<State,Action,_>::rebuild(&self.label.li8,&prev.label.li8,state,ctx, widget9::ButtonL9::label8_mut(&mut el));});
    ctx.with_id(id_lvw9, |ctx|{View::<State,Action,_>::rebuild(&self.label.lj9,&prev.label.lj9,state,ctx, widget9::ButtonL9::label9_mut(&mut el));});

    // rebuild based on LabelOpt, do manuall diff for each prop
    if prev.opt.pad.tl1 != self.opt.pad.tl1 {widget9::ButtonL9::set_pad1 (&mut el, self.opt.pad.tl1);}
    if prev.opt.pad.ti2 != self.opt.pad.ti2 {widget9::ButtonL9::set_pad2 (&mut el, self.opt.pad.ti2);}
    if prev.opt.pad.tj3 != self.opt.pad.tj3 {widget9::ButtonL9::set_pad3 (&mut el, self.opt.pad.tj3);}
    if prev.opt.pad.hl4 != self.opt.pad.hl4 {widget9::ButtonL9::set_pad4 (&mut el, self.opt.pad.hl4);}
    if prev.opt.pad.hi5 != self.opt.pad.hi5 {widget9::ButtonL9::set_pad5 (&mut el, self.opt.pad.hi5);}
    if prev.opt.pad.hj6 != self.opt.pad.hj6 {widget9::ButtonL9::set_pad6 (&mut el, self.opt.pad.hj6);}
    if prev.opt.pad.ll7 != self.opt.pad.ll7 {widget9::ButtonL9::set_pad7 (&mut el, self.opt.pad.ll7);}
    if prev.opt.pad.li8 != self.opt.pad.li8 {widget9::ButtonL9::set_pad8 (&mut el, self.opt.pad.li8);}
    if prev.opt.pad.lj9 != self.opt.pad.lj9 {widget9::ButtonL9::set_pad9 (&mut el, self.opt.pad.lj9);}
  }

  fn teardown(&self, _:&mut Self::ViewState, ctx:&mut ViewCtx, mut el:Mut<Self::Element>) {
    // teardown LabelViews, which already implement teardown themselves
    ctx.with_id(id_lvw1, |ctx|{View::<State,Action,_>::teardown(&self.label.tl1,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw2, |ctx|{View::<State,Action,_>::teardown(&self.label.ti2,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw3, |ctx|{View::<State,Action,_>::teardown(&self.label.tj3,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw4, |ctx|{View::<State,Action,_>::teardown(&self.label.hl4,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw5, |ctx|{View::<State,Action,_>::teardown(&self.label.hi5,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw6, |ctx|{View::<State,Action,_>::teardown(&self.label.hj6,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw7, |ctx|{View::<State,Action,_>::teardown(&self.label.ll7,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw8, |ctx|{View::<State,Action,_>::teardown(&self.label.li8,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw9, |ctx|{View::<State,Action,_>::teardown(&self.label.lj9,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.teardown_leaf(el); // teardown the button element itself
  }

  fn message(&self, _:&mut Self::ViewState, id_path:&[ViewId], message:DynMessage, app_state:&mut State) -> MsgRes<Action> {
    match id_path.split_first() {
      Some((&id_lvw1, rest)) => self.label.tl1.message(&mut(), rest, message, app_state),
      Some((&id_lvw2, rest)) => self.label.ti2.message(&mut(), rest, message, app_state),
      Some((&id_lvw3, rest)) => self.label.tj3.message(&mut(), rest, message, app_state),
      Some((&id_lvw4, rest)) => self.label.hl4.message(&mut(), rest, message, app_state),
      Some((&id_lvw5, rest)) => self.label.hi5.message(&mut(), rest, message, app_state),
      Some((&id_lvw6, rest)) => self.label.hj6.message(&mut(), rest, message, app_state),
      Some((&id_lvw7, rest)) => self.label.ll7.message(&mut(), rest, message, app_state),
      Some((&id_lvw8, rest)) => self.label.li8.message(&mut(), rest, message, app_state),
      Some((&id_lvw9, rest)) => self.label.lj9.message(&mut(), rest, message, app_state),
      None => match message.downcast::<masonry::core::Action>() {
        Ok(action)   => {
          if let masonry::core::Action::ButtonPressed(button) = *action {
            (self.callback)(app_state, button)
          } else        {tracing::error!("Wrong action type in ButtonL9::message: {action:?}");
            MessageResult::Stale(action)} }
        Err(message) => {tracing::error!("Wrong message type in ButtonL9::message: {message:?}");
            MessageResult::Stale(message) }   },
      _    =>           {tracing::warn! ("Got unexpected ID path in ButtonL9::message");
            MessageResult::Stale(message)      }
    }
  }
}
