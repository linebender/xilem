// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Helper types for test writing.
//!
//! This includes tools for making throwaway widgets more easily.
//!
//! Note: Some of these types are undocumented. They're meant to help maintainers of Druid and
//! people trying to build a framework on top of Druid (like crochet), not to be user-facing.

#![allow(missing_docs)]

use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::*;
use crate::event::StatusChange;

pub type EventFn<S> = dyn FnMut(&mut S, &mut EventCtx, &Event, &Env);
pub type StatusChangeFn<S> = dyn FnMut(&mut S, &mut LifeCycleCtx, &StatusChange, &Env);
pub type LifeCycleFn<S> = dyn FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle, &Env);
pub type LayoutFn<S> = dyn FnMut(&mut S, &mut LayoutCtx, &BoxConstraints, &Env) -> Size;
pub type PaintFn<S> = dyn FnMut(&mut S, &mut PaintCtx, &Env);
pub type ChildrenFn<S> = dyn Fn(&S) -> SmallVec<[&dyn AsWidgetPod; 16]>;
pub type ChildrenMutFn<S> = dyn FnMut(&mut S) -> SmallVec<[&mut dyn AsWidgetPod; 16]>;

pub const REPLACE_CHILD: Selector = Selector::new("druid-test.replace-child");

/// A widget that can be constructed from individual functions, builder-style.
///
/// This widget is generic over its state, which is passed in at construction time.
pub struct ModularWidget<S> {
    state: S,
    on_event: Option<Box<EventFn<S>>>,
    on_status_change: Option<Box<StatusChangeFn<S>>>,
    lifecycle: Option<Box<LifeCycleFn<S>>>,
    layout: Option<Box<LayoutFn<S>>>,
    paint: Option<Box<PaintFn<S>>>,
    children: Option<Box<ChildrenFn<S>>>,
    children_mut: Option<Box<ChildrenMutFn<S>>>,
}

/// A widget that can replace its child on command
pub struct ReplaceChild {
    child: WidgetPod<Box<dyn Widget>>,
    replacer: Box<dyn Fn() -> WidgetPod<Box<dyn Widget>>>,
}

/// A widget that records each time one of its methods is called.
///
/// Make one like this:
///
/// ```
/// # use druid::widget::Label;
/// # use druid::{WidgetExt, LifeCycle};
/// use druid::tests::helpers::{Recording, Record, TestWidgetExt};
/// use druid::tests::harness::Harness;
/// let recording = Recording::default();
/// let widget = Label::new("Hello").padding(4.0).record(&recording);
///
/// Harness::create_simple((), widget, |harness| {
///     harness.send_initial_events();
///     assert!(matches!(recording.next(), Record::L(LifeCycle::WidgetAdded)));
/// })
/// ```
pub struct Recorder<W> {
    recording: Recording,
    child: W,
}

/// A recording of widget method calls.
#[derive(Debug, Clone, Default)]
pub struct Recording(Rc<RefCell<VecDeque<Record>>>);

/// A recording of a method call on a widget.
///
/// Each member of the enum corresponds to one of the methods on `Widget`.
#[derive(Debug, Clone)]
pub enum Record {
    /// An `Event`.
    E(Event),
    SC(StatusChange),
    /// A `LifeCycle` event.
    L(LifeCycle),
    Layout(Size),
    Paint,
    // instead of always returning an Option<Record>, we have a none variant;
    // this would be code smell elsewhere but here I think it makes the tests
    // easier to read.
    None,
}

/// like WidgetExt but just for this one thing
pub trait TestWidgetExt: Widget + Sized + 'static {
    fn record(self, recording: &Recording) -> Recorder<Self> {
        Recorder {
            child: self,
            recording: recording.clone(),
        }
    }
}

impl<W: Widget + 'static> TestWidgetExt for W {}

impl<S> ModularWidget<S> {
    pub fn new(state: S) -> Self {
        ModularWidget {
            state,
            on_event: None,
            on_status_change: None,
            lifecycle: None,
            layout: None,
            paint: None,
            children: None,
            children_mut: None,
        }
    }

    pub fn event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx, &Event, &Env) + 'static,
    ) -> Self {
        self.on_event = Some(Box::new(f));
        self
    }

    pub fn status_change_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LifeCycleCtx, &StatusChange, &Env) + 'static,
    ) -> Self {
        self.on_status_change = Some(Box::new(f));
        self
    }

    pub fn lifecycle_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle, &Env) + 'static,
    ) -> Self {
        self.lifecycle = Some(Box::new(f));
        self
    }

    pub fn layout_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LayoutCtx, &BoxConstraints, &Env) -> Size + 'static,
    ) -> Self {
        self.layout = Some(Box::new(f));
        self
    }

    pub fn paint_fn(mut self, f: impl FnMut(&mut S, &mut PaintCtx, &Env) + 'static) -> Self {
        self.paint = Some(Box::new(f));
        self
    }

    pub fn children_fns(
        mut self,
        children: impl Fn(&S) -> SmallVec<[&dyn AsWidgetPod; 16]> + 'static,
        children_mut: impl FnMut(&mut S) -> SmallVec<[&mut dyn AsWidgetPod; 16]> + 'static,
    ) -> Self {
        self.children = Some(Box::new(children));
        self.children_mut = Some(Box::new(children_mut));
        self
    }
}

impl<S> Widget for ModularWidget<S> {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        if let Some(f) = self.on_event.as_mut() {
            f(&mut self.state, ctx, event, env)
        }
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env) {
        if let Some(f) = self.on_status_change.as_mut() {
            f(&mut self.state, ctx, event, env)
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        if let Some(f) = self.lifecycle.as_mut() {
            f(&mut self.state, ctx, event, env)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        let ModularWidget {
            ref mut state,
            ref mut layout,
            ..
        } = self;
        layout
            .as_mut()
            .map(|f| f(state, ctx, bc, env))
            .unwrap_or_else(|| Size::new(100., 100.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        if let Some(f) = self.paint.as_mut() {
            f(&mut self.state, ctx, env)
        }
    }

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]> {
        if let Some(f) = self.children.as_ref() {
            f(&self.state)
        } else {
            SmallVec::new()
        }
    }

    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]> {
        if let Some(f) = self.children_mut.as_mut() {
            f(&mut self.state)
        } else {
            SmallVec::new()
        }
    }
}

impl ReplaceChild {
    pub fn new<W: Widget + 'static>(
        child: impl Widget + 'static,
        f: impl Fn() -> W + 'static,
    ) -> Self {
        let child = WidgetPod::new(child).boxed();
        let replacer = Box::new(move || WidgetPod::new(f()).boxed());
        ReplaceChild { child, replacer }
    }
}

impl Widget for ReplaceChild {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        // TODO
        /*
        if let Event::Command(cmd) = event {
            if cmd.is(REPLACE_CHILD) {
                self.child = WidgetPod::new((self.replacer)());
                ctx.children_changed();
                return;
            }
        }
        */
        self.child.on_event(ctx, event, env)
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env) {
        ctx.request_layout();
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        self.child.lifecycle(ctx, event, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        self.child.layout(ctx, bc, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        self.child.paint_raw(ctx, env)
    }

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]> {
        self.child.widget().children()
    }

    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]> {
        self.child.widget_mut().children_mut()
    }
}

#[allow(dead_code)]
impl Recording {
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear()
    }

    /// Returns the next event in the recording, if one exists.
    ///
    /// This consumes the event.
    pub fn next(&self) -> Record {
        self.0.borrow_mut().pop_front().unwrap_or(Record::None)
    }

    /// Returns an iterator of events drained from the recording.
    pub fn drain(&self) -> impl Iterator<Item = Record> {
        self.0
            .borrow_mut()
            .drain(..)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn push(&self, event: Record) {
        self.0.borrow_mut().push_back(event)
    }
}

impl<W: Widget> Widget for Recorder<W> {
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        self.recording.push(Record::E(event.clone()));
        self.child.on_event(ctx, event, env)
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange, env: &Env) {
        self.recording.push(Record::SC(event.clone()));
        self.child.on_status_change(ctx, event, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        self.recording.push(Record::L(event.clone()));
        self.child.lifecycle(ctx, event, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, env);
        self.recording.push(Record::Layout(size));
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        self.child.paint(ctx, env);
        self.recording.push(Record::Paint)
    }

    fn children(&self) -> SmallVec<[&dyn AsWidgetPod; 16]> {
        self.child.children()
    }

    fn children_mut(&mut self) -> SmallVec<[&mut dyn AsWidgetPod; 16]> {
        self.child.children_mut()
    }
}
