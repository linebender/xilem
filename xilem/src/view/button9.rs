// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::view::PointerButton;
pub use masonry::core::WidgetPod;
use masonry::widgets::{self, LabelOpt, Pad9};

use crate::core::{
    DynMessage, MessageResult, MessageResult as MsgRes, Mut, View, ViewId, ViewMarker,
    ViewPathTracker,
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
pub fn button9<State, Action>(
    label: impl Into<Label>,
    callback: impl Fn(&mut State) -> Action + Send + 'static,
) -> Button9<impl for<'a> Fn(&'a mut State, PointerButton) -> MsgRes<Action> + Send + 'static> {
    button9_pad(label, None, callback)
}
/// A button with custom `pad` padding which calls `callback` when 🖰1 (normally left) is pressed
pub fn button9_pad<State, Action>(
    label: impl Into<Label>,
    pad: Option<Insets>,
    callback: impl Fn(&mut State) -> Action + Send + 'static,
) -> Button9<impl for<'a> Fn(&'a mut State, PointerButton) -> MsgRes<Action> + Send + 'static> {
    Button9::new(label, pad, move |state: &mut State, button| match button {
        PointerButton::Primary => MsgRes::Action(callback(state)),
        _ => MsgRes::Nop,
    })
}
/// A button which calls `callback` when any 🖰 is pressed
pub fn button9_any_pointer<State, Action>(
    label: impl Into<Label>,
    callback: impl Fn(&mut State, PointerButton) -> Action + Send + 'static,
) -> Button9<impl for<'a> Fn(&'a mut State, PointerButton) -> MsgRes<Action> + Send + 'static> {
    button9_any_pointer_pad(label, None, callback)
}
/// A button with custom `pad` padding which calls `callback` when any 🖰 is pressed
pub fn button9_any_pointer_pad<State, Action>(
    label: impl Into<Label>,
    pad: Option<Insets>,
    callback: impl Fn(&mut State, PointerButton) -> Action + Send + 'static,
) -> Button9<impl for<'a> Fn(&'a mut State, PointerButton) -> MsgRes<Action> + Send + 'static> {
    Button9::new(label, pad, move |state: &mut State, button| {
        MsgRes::Action(callback(state, button))
    })
}

/// The [`View`] created by [`button9`] from up to label(s) in one of 9 positions with custom padding and a callback.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Button9<F> {
    label: Label9,
    opt: LabelOpt,
    callback: F,
}
/// Label for Button9
pub struct Label9 {
    p1: Label,
    p2: Label,
    p3: Label, // ↖  ↑  ↗
    p4: Label,
    p5: Label,
    p6: Label, // ←  •  →
    p7: Label,
    p8: Label,
    p9: Label, // ↙  ↓  ↘
}

impl<F> Button9<F> {
    /// Create a new button with a text label at the center
    /// ([`Label9`].p5, others 8 labels are blank by default, use [`Button9::add1`]–[`Button9::add9`] methods to fill them)
    pub fn new(label: impl Into<Label>, pad: Option<Insets>, callback: F) -> Self {
        let label = Label9 {
            p1: "".into(),
            p2: "".into(),
            p3: "".into(), // ↖  ↑  ↗
            p4: "".into(),
            p5: label.into(),
            p6: "".into(), // ←  •  →
            p7: "".into(),
            p8: "".into(),
            p9: "".into(), // ↙  ↓  ↘
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
        Self {
            label,
            opt,
            callback,
        }
    }
    /// Add label at •p5
    pub fn add(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p5 = label.into();
        self.opt.pad.p5 = pad;
        self
    }
    /// Add label at ↖p1
    pub fn add1(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p1 = label.into();
        self.opt.pad.p1 = pad;
        self
    }
    /// Add label at ↑p2
    pub fn add2(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p2 = label.into();
        self.opt.pad.p2 = pad;
        self
    }
    /// Add label at ↗p3
    pub fn add3(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p3 = label.into();
        self.opt.pad.p3 = pad;
        self
    }
    /// Add label at ←p4
    pub fn add4(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p4 = label.into();
        self.opt.pad.p4 = pad;
        self
    }
    /// Add label at •p5
    pub fn add5(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p5 = label.into();
        self.opt.pad.p5 = pad;
        self
    }
    /// Add label at →p6
    pub fn add6(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p6 = label.into();
        self.opt.pad.p6 = pad;
        self
    }
    /// Add label at ↙p7
    pub fn add7(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p7 = label.into();
        self.opt.pad.p7 = pad;
        self
    }
    /// Add label at ↓p8
    pub fn add8(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p8 = label.into();
        self.opt.pad.p8 = pad;
        self
    }
    /// Add label at ↘p9
    pub fn add9(mut self, label: impl Into<Label>, pad: Option<Insets>) -> Self {
        self.label.p9 = label.into();
        self.opt.pad.p9 = pad;
        self
    }
}

const ID_LVW1: ViewId = ViewId::new(1);
const ID_LVW2: ViewId = ViewId::new(2);
const ID_LVW3: ViewId = ViewId::new(3);
const ID_LVW4: ViewId = ViewId::new(4);
const ID_LVW5: ViewId = ViewId::new(5);
const ID_LVW6: ViewId = ViewId::new(6);
const ID_LVW7: ViewId = ViewId::new(7);
const ID_LVW8: ViewId = ViewId::new(8);
const ID_LVW9: ViewId = ViewId::new(9);

impl<F> ViewMarker for Button9<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for Button9<F>
where
    F: Fn(&mut State, PointerButton) -> MsgRes<Action> + Send + Sync + 'static,
{
    type Element = Pod<widgets::Button9>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        // build based on LabelViews, which already implement build themselves:(Self::Element, Self::ViewState)
        let (child1, ()) = ctx.with_id(ID_LVW1, |ctx| {
            View::<State, Action, _>::build(&self.label.p1, ctx)
        });
        let (child2, ()) = ctx.with_id(ID_LVW2, |ctx| {
            View::<State, Action, _>::build(&self.label.p2, ctx)
        });
        let (child3, ()) = ctx.with_id(ID_LVW3, |ctx| {
            View::<State, Action, _>::build(&self.label.p3, ctx)
        });
        let (child4, ()) = ctx.with_id(ID_LVW4, |ctx| {
            View::<State, Action, _>::build(&self.label.p4, ctx)
        });
        let (child5, ()) = ctx.with_id(ID_LVW5, |ctx| {
            View::<State, Action, _>::build(&self.label.p5, ctx)
        });
        let (child6, ()) = ctx.with_id(ID_LVW6, |ctx| {
            View::<State, Action, _>::build(&self.label.p6, ctx)
        });
        let (child7, ()) = ctx.with_id(ID_LVW7, |ctx| {
            View::<State, Action, _>::build(&self.label.p7, ctx)
        });
        let (child8, ()) = ctx.with_id(ID_LVW8, |ctx| {
            View::<State, Action, _>::build(&self.label.p8, ctx)
        });
        let (child9, ()) = ctx.with_id(ID_LVW9, |ctx| {
            View::<State, Action, _>::build(&self.label.p9, ctx)
        });
        // pass built elements to the masonry widgets
        let label = [
            child1.into_widget_pod(),
            child2.into_widget_pod(),
            child3.into_widget_pod(), // ↖  ↑  ↗
            child4.into_widget_pod(),
            child5.into_widget_pod(),
            child8.into_widget_pod(), // ←  •  →
            child7.into_widget_pod(),
            child6.into_widget_pod(),
            child9.into_widget_pod(), // ↙  ↓  ↘
        ];
        let pad = Pad9 {
            p1: self.opt.pad.p1,
            p2: self.opt.pad.p2,
            p3: self.opt.pad.p3, // ↖  ↑  ↗
            p4: self.opt.pad.p4,
            p5: self.opt.pad.p5,
            p6: self.opt.pad.p6, // ←  •  →
            p7: self.opt.pad.p7,
            p8: self.opt.pad.p8,
            p9: self.opt.pad.p9, // ↙  ↓  ↘
        };
        ctx.with_leaf_action_widget(|ctx| ctx.new_pod(widgets::Button9::from_label_pod(label, pad)))
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut el: Mut<Self::Element>,
    ) {
        // rebuild based on LabelViews, which already implement rebuild themselves (compare all the props)
        ctx.with_id(ID_LVW1, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p1,
                &prev.label.p1,
                state,
                ctx,
                widgets::Button9::label1_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW2, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p2,
                &prev.label.p2,
                state,
                ctx,
                widgets::Button9::label2_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW3, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p3,
                &prev.label.p3,
                state,
                ctx,
                widgets::Button9::label3_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW4, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p4,
                &prev.label.p4,
                state,
                ctx,
                widgets::Button9::label4_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW5, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p5,
                &prev.label.p5,
                state,
                ctx,
                widgets::Button9::label5_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW6, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p6,
                &prev.label.p6,
                state,
                ctx,
                widgets::Button9::label6_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW7, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p7,
                &prev.label.p7,
                state,
                ctx,
                widgets::Button9::label7_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW8, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p8,
                &prev.label.p8,
                state,
                ctx,
                widgets::Button9::label8_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW9, |ctx| {
            View::<State, Action, _>::rebuild(
                &self.label.p9,
                &prev.label.p9,
                state,
                ctx,
                widgets::Button9::label9_mut(&mut el),
            );
        });

        // rebuild based on LabelOpt, do manuall diff for each prop
        if prev.opt.pad.p1 != self.opt.pad.p1 {
            widgets::Button9::set_pad1(&mut el, self.opt.pad.p1);
        }
        if prev.opt.pad.p2 != self.opt.pad.p2 {
            widgets::Button9::set_pad2(&mut el, self.opt.pad.p2);
        }
        if prev.opt.pad.p3 != self.opt.pad.p3 {
            widgets::Button9::set_pad3(&mut el, self.opt.pad.p3);
        }
        if prev.opt.pad.p4 != self.opt.pad.p4 {
            widgets::Button9::set_pad4(&mut el, self.opt.pad.p4);
        }
        if prev.opt.pad.p5 != self.opt.pad.p5 {
            widgets::Button9::set_pad5(&mut el, self.opt.pad.p5);
        }
        if prev.opt.pad.p6 != self.opt.pad.p6 {
            widgets::Button9::set_pad6(&mut el, self.opt.pad.p6);
        }
        if prev.opt.pad.p7 != self.opt.pad.p7 {
            widgets::Button9::set_pad7(&mut el, self.opt.pad.p7);
        }
        if prev.opt.pad.p8 != self.opt.pad.p8 {
            widgets::Button9::set_pad8(&mut el, self.opt.pad.p8);
        }
        if prev.opt.pad.p9 != self.opt.pad.p9 {
            widgets::Button9::set_pad9(&mut el, self.opt.pad.p9);
        }
    }

    fn teardown(&self, _: &mut Self::ViewState, ctx: &mut ViewCtx, mut el: Mut<Self::Element>) {
        // teardown LabelViews, which already implement teardown themselves
        ctx.with_id(ID_LVW1, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p1,
                &mut (),
                ctx,
                widgets::Button9::label1_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW2, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p2,
                &mut (),
                ctx,
                widgets::Button9::label2_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW3, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p3,
                &mut (),
                ctx,
                widgets::Button9::label3_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW4, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p4,
                &mut (),
                ctx,
                widgets::Button9::label4_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW5, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p5,
                &mut (),
                ctx,
                widgets::Button9::label5_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW6, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p6,
                &mut (),
                ctx,
                widgets::Button9::label6_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW7, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p7,
                &mut (),
                ctx,
                widgets::Button9::label7_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW8, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p8,
                &mut (),
                ctx,
                widgets::Button9::label8_mut(&mut el),
            );
        });
        ctx.with_id(ID_LVW9, |ctx| {
            View::<State, Action, _>::teardown(
                &self.label.p9,
                &mut (),
                ctx,
                widgets::Button9::label9_mut(&mut el),
            );
        });
        ctx.teardown_leaf(el); // teardown the button element itself
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MsgRes<Action> {
        match id_path.split_first() {
            Some((&ID_LVW1, rest)) => self.label.p1.message(&mut (), rest, message, app_state),
            Some((&ID_LVW2, rest)) => self.label.p2.message(&mut (), rest, message, app_state),
            Some((&ID_LVW3, rest)) => self.label.p3.message(&mut (), rest, message, app_state),
            Some((&ID_LVW4, rest)) => self.label.p4.message(&mut (), rest, message, app_state),
            Some((&ID_LVW5, rest)) => self.label.p5.message(&mut (), rest, message, app_state),
            Some((&ID_LVW6, rest)) => self.label.p6.message(&mut (), rest, message, app_state),
            Some((&ID_LVW7, rest)) => self.label.p7.message(&mut (), rest, message, app_state),
            Some((&ID_LVW8, rest)) => self.label.p8.message(&mut (), rest, message, app_state),
            Some((&ID_LVW9, rest)) => self.label.p9.message(&mut (), rest, message, app_state),
            None => match message.downcast::<masonry::core::Action>() {
                Ok(action) => {
                    if let masonry::core::Action::ButtonPressed(button) = *action {
                        (self.callback)(app_state, button)
                    } else {
                        tracing::error!("Wrong action type in Button9::message: {action:?}");
                        MessageResult::Stale(action)
                    }
                }
                Err(message) => {
                    tracing::error!("Wrong message type in Button9::message: {message:?}");
                    MessageResult::Stale(message)
                }
            },
            _ => {
                tracing::warn!("Got unexpected ID path in Button9::message");
                MessageResult::Stale(message)
            }
        }
    }
}
