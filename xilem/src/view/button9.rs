// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

pub use masonry::core::{PointerButton, PointerButton as PointerB, WidgetPod};
use masonry::widgets;
use crate::binmod::custom_button_masonry as widget9;

use xilem::core::{ViewPathTracker,
  DynMessage, Mut, View, ViewMarker,
  MessageResult, ViewId, MessageResult as MsgRes,
};
use xilem::view::Label;
use xilem::{Pod, ViewCtx};

use masonry::kurbo::{Insets};

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

use crate::binmod::custom_button_masonry::{LPos, LabelOpt, Pad9};

/// The [`View`] created by [`button`] from up to label(s) in one of [`LPos`] position with custom padding and a callback.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ButtonL9<F> {
  label: Label9,
  opt  : LabelOpt,
  callback: F,
}
/// Label for ButtonL9
pub struct Label9 {
  TL1:Label, TI2:Label, TJ3:Label, // ↖  ↑  ↗
  HL4:Label, HI5:Label, HJ6:Label, // ←  •  →
  LL7:Label, LI8:Label, LJ9:Label, // ↙  ↓  ↘
}

impl<F> ButtonL9<F>{
  /// Create a new button with a text label at the center (HI5m other labels are blank, use `.addx` methods to fill them)
  pub fn new(                     label:impl Into<Label>, pad:Option<Insets>, callback:F) -> Self {
    let label = Label9 {
      TL1:"".into(), TI2:""   .into(), TJ3:"".into(), // ↖  ↑  ↗
      HL4:"".into(), HI5:label.into(), HJ6:"".into(), // ←  •  →
      LL7:"".into(), LI8:""   .into(), LJ9:"".into(), // ↙  ↓  ↘
    };
    let pad = Pad9 {
      TL1:None, TI2:None, TJ3:None, // ↖  ↑  ↗
      HL4:None, HI5:pad , HJ6:None, // ←  •  →
      LL7:None, LI8:None, LJ9:None, // ↙  ↓  ↘
    };
    let opt = LabelOpt{pad};
    Self {label, opt, callback}
  }
  /// Helper .methods for adding individual labels (add=center HI5)
  pub fn add (mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.HI5 = label.into(); self.opt.pad.HI5 = pad; self}
  pub fn add1(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.TL1 = label.into(); self.opt.pad.TL1 = pad; self}
  pub fn add2(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.TI2 = label.into(); self.opt.pad.TI2 = pad; self}
  pub fn add3(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.TJ3 = label.into(); self.opt.pad.TJ3 = pad; self}
  pub fn add4(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.HL4 = label.into(); self.opt.pad.HL4 = pad; self}
  pub fn add5(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.HI5 = label.into(); self.opt.pad.HI5 = pad; self}
  pub fn add6(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.HJ6 = label.into(); self.opt.pad.HJ6 = pad; self}
  pub fn add7(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.LL7 = label.into(); self.opt.pad.LL7 = pad; self}
  pub fn add8(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.LI8 = label.into(); self.opt.pad.LI8 = pad; self}
  pub fn add9(mut self,         label:impl Into<Label>, pad:Option<Insets>) -> Self {self.label.LJ9 = label.into(); self.opt.pad.LJ9 = pad; self}
  // pub fn add (mut self,         label:impl Into<Label>, pad:Option<Insets>) {self.addx(LPos::HI5,label,pad)}
  /// Helper .method for adding a label to a given position (same as in [`LPos`])
  pub fn addx(mut self,idx:LPos,label:impl Into<Label>, pad:Option<Insets>) -> Self {match idx {
    LPos::TL1 => {self.label.TL1 = label.into(); self.opt.pad.TL1 = pad}, //↖
    LPos::TI2 => {self.label.TI2 = label.into(); self.opt.pad.TI2 = pad}, //↑
    LPos::TJ3 => {self.label.TJ3 = label.into(); self.opt.pad.TJ3 = pad}, //↗
    LPos::HL4 => {self.label.HL4 = label.into(); self.opt.pad.HL4 = pad}, //←
    LPos::HI5 => {self.label.HI5 = label.into(); self.opt.pad.HI5 = pad}, //•
    LPos::HJ6 => {self.label.HJ6 = label.into(); self.opt.pad.HJ6 = pad}, //→
    LPos::LL7 => {self.label.LL7 = label.into(); self.opt.pad.LL7 = pad}, //↙
    LPos::LI8 => {self.label.LI8 = label.into(); self.opt.pad.LI8 = pad}, //↓
    LPos::LJ9 => {self.label.LJ9 = label.into(); self.opt.pad.LJ9 = pad}, //↘
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
    let (child1,()) = ctx.with_id(id_lvw1, |ctx|{View::<State,Action,_>::build(&self.label.TL1,ctx)});
    let (child2,()) = ctx.with_id(id_lvw2, |ctx|{View::<State,Action,_>::build(&self.label.TI2,ctx)});
    let (child3,()) = ctx.with_id(id_lvw3, |ctx|{View::<State,Action,_>::build(&self.label.TJ3,ctx)});
    let (child4,()) = ctx.with_id(id_lvw4, |ctx|{View::<State,Action,_>::build(&self.label.HL4,ctx)});
    let (child5,()) = ctx.with_id(id_lvw5, |ctx|{View::<State,Action,_>::build(&self.label.HI5,ctx)});
    let (child6,()) = ctx.with_id(id_lvw6, |ctx|{View::<State,Action,_>::build(&self.label.HJ6,ctx)});
    let (child7,()) = ctx.with_id(id_lvw7, |ctx|{View::<State,Action,_>::build(&self.label.LL7,ctx)});
    let (child8,()) = ctx.with_id(id_lvw8, |ctx|{View::<State,Action,_>::build(&self.label.LI8,ctx)});
    let (child9,()) = ctx.with_id(id_lvw9, |ctx|{View::<State,Action,_>::build(&self.label.LJ9,ctx)});
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
      TL1:self.opt.pad.TL1.clone(), TI2:self.opt.pad.TI2.clone(), TJ3:self.opt.pad.TJ3.clone(), // ↖  ↑  ↗
      HL4:self.opt.pad.HL4.clone(), HI5:self.opt.pad.HI5.clone(), HJ6:self.opt.pad.HJ6.clone(), // ←  •  →
      LL7:self.opt.pad.LL7.clone(), LI8:self.opt.pad.LI8.clone(), LJ9:self.opt.pad.LJ9.clone(), // ↙  ↓  ↘
    };
    ctx.with_leaf_action_widget(|ctx| {ctx.new_pod(widget9::ButtonL9::from_label_pod(label,pad)) } )
  }

  fn rebuild(&self, prev:&Self, state:&mut Self::ViewState, ctx:&mut ViewCtx, mut el:Mut<Self::Element>) {
    // rebuild based on LabelViews, which already implement rebuild themselves (compare all the props)
    ctx.with_id(id_lvw1, |ctx|{View::<State,Action,_>::rebuild(&self.label.TL1,&prev.label.TL1,state,ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw2, |ctx|{View::<State,Action,_>::rebuild(&self.label.TI2,&prev.label.TI2,state,ctx, widget9::ButtonL9::label2_mut(&mut el));});
    ctx.with_id(id_lvw3, |ctx|{View::<State,Action,_>::rebuild(&self.label.TJ3,&prev.label.TJ3,state,ctx, widget9::ButtonL9::label3_mut(&mut el));});
    ctx.with_id(id_lvw4, |ctx|{View::<State,Action,_>::rebuild(&self.label.HL4,&prev.label.HL4,state,ctx, widget9::ButtonL9::label4_mut(&mut el));});
    ctx.with_id(id_lvw5, |ctx|{View::<State,Action,_>::rebuild(&self.label.HI5,&prev.label.HI5,state,ctx, widget9::ButtonL9::label5_mut(&mut el));});
    ctx.with_id(id_lvw6, |ctx|{View::<State,Action,_>::rebuild(&self.label.HJ6,&prev.label.HJ6,state,ctx, widget9::ButtonL9::label6_mut(&mut el));});
    ctx.with_id(id_lvw7, |ctx|{View::<State,Action,_>::rebuild(&self.label.LL7,&prev.label.LL7,state,ctx, widget9::ButtonL9::label7_mut(&mut el));});
    ctx.with_id(id_lvw8, |ctx|{View::<State,Action,_>::rebuild(&self.label.LI8,&prev.label.LI8,state,ctx, widget9::ButtonL9::label8_mut(&mut el));});
    ctx.with_id(id_lvw9, |ctx|{View::<State,Action,_>::rebuild(&self.label.LJ9,&prev.label.LJ9,state,ctx, widget9::ButtonL9::label9_mut(&mut el));});

    // rebuild based on LabelOpt, do manuall diff for each prop
    if prev.opt.pad.TL1 != self.opt.pad.TL1 {widget9::ButtonL9::set_pad1 (&mut el, self.opt.pad.TL1);}
    if prev.opt.pad.TI2 != self.opt.pad.TI2 {widget9::ButtonL9::set_pad2 (&mut el, self.opt.pad.TI2);}
    if prev.opt.pad.TJ3 != self.opt.pad.TJ3 {widget9::ButtonL9::set_pad3 (&mut el, self.opt.pad.TJ3);}
    if prev.opt.pad.HL4 != self.opt.pad.HL4 {widget9::ButtonL9::set_pad4 (&mut el, self.opt.pad.HL4);}
    if prev.opt.pad.HI5 != self.opt.pad.HI5 {widget9::ButtonL9::set_pad5 (&mut el, self.opt.pad.HI5);}
    if prev.opt.pad.HJ6 != self.opt.pad.HJ6 {widget9::ButtonL9::set_pad6 (&mut el, self.opt.pad.HJ6);}
    if prev.opt.pad.LL7 != self.opt.pad.LL7 {widget9::ButtonL9::set_pad7 (&mut el, self.opt.pad.LL7);}
    if prev.opt.pad.LI8 != self.opt.pad.LI8 {widget9::ButtonL9::set_pad8 (&mut el, self.opt.pad.LI8);}
    if prev.opt.pad.LJ9 != self.opt.pad.LJ9 {widget9::ButtonL9::set_pad9 (&mut el, self.opt.pad.LJ9);}
  }

  fn teardown(&self, _:&mut Self::ViewState, ctx:&mut ViewCtx, mut el:Mut<Self::Element>) {
    // teardown LabelViews, which already implement teardown themselves
    ctx.with_id(id_lvw1, |ctx|{View::<State,Action,_>::teardown(&self.label.TL1,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw2, |ctx|{View::<State,Action,_>::teardown(&self.label.TI2,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw3, |ctx|{View::<State,Action,_>::teardown(&self.label.TJ3,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw4, |ctx|{View::<State,Action,_>::teardown(&self.label.HL4,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw5, |ctx|{View::<State,Action,_>::teardown(&self.label.HI5,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw6, |ctx|{View::<State,Action,_>::teardown(&self.label.HJ6,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw7, |ctx|{View::<State,Action,_>::teardown(&self.label.LL7,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw8, |ctx|{View::<State,Action,_>::teardown(&self.label.LI8,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.with_id(id_lvw9, |ctx|{View::<State,Action,_>::teardown(&self.label.LJ9,&mut (),ctx, widget9::ButtonL9::label1_mut(&mut el));});
    ctx.teardown_leaf(el); // teardown the button element itself
  }

  fn message(&self, _:&mut Self::ViewState, id_path:&[ViewId], message:DynMessage, app_state:&mut State) -> MsgRes<Action> {
    match id_path.split_first() {
      Some((&id_lvw1, rest)) => self.label.TL1.message(&mut(), rest, message, app_state),
      Some((&id_lvw2, rest)) => self.label.TI2.message(&mut(), rest, message, app_state),
      Some((&id_lvw3, rest)) => self.label.TJ3.message(&mut(), rest, message, app_state),
      Some((&id_lvw4, rest)) => self.label.HL4.message(&mut(), rest, message, app_state),
      Some((&id_lvw5, rest)) => self.label.HI5.message(&mut(), rest, message, app_state),
      Some((&id_lvw6, rest)) => self.label.HJ6.message(&mut(), rest, message, app_state),
      Some((&id_lvw7, rest)) => self.label.LL7.message(&mut(), rest, message, app_state),
      Some((&id_lvw8, rest)) => self.label.LI8.message(&mut(), rest, message, app_state),
      Some((&id_lvw9, rest)) => self.label.LJ9.message(&mut(), rest, message, app_state),
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
