// Copyright 2020 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper widgets for writing tests.
//!
//! This includes tools for making throwaway widgets more easily.
//!
//! Note: Some of these types are undocumented. They're meant to help maintainers of
//! Masonry, not to be user-facing.

#![allow(missing_docs)]
#![allow(unused)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use accesskit::{NodeBuilder, Role};
use accesskit_winit::Event;
use smallvec::SmallVec;
use vello::Scene;

use crate::event::{PointerEvent, TextEvent};
use crate::widget::SizedBox;
use crate::*;

pub type PointerEventFn<S> = dyn FnMut(&mut S, &mut EventCtx, &PointerEvent);
pub type TextEventFn<S> = dyn FnMut(&mut S, &mut EventCtx, &TextEvent);
pub type AccessEventFn<S> = dyn FnMut(&mut S, &mut EventCtx, &AccessEvent);
pub type RegisterChildrenFn<S> = dyn FnMut(&mut S, &mut RegisterCtx);
pub type StatusChangeFn<S> = dyn FnMut(&mut S, &mut LifeCycleCtx, &StatusChange);
pub type LifeCycleFn<S> = dyn FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle);
pub type LayoutFn<S> = dyn FnMut(&mut S, &mut LayoutCtx, &BoxConstraints) -> Size;
pub type ComposeFn<S> = dyn FnMut(&mut S, &mut ComposeCtx);
pub type PaintFn<S> = dyn FnMut(&mut S, &mut PaintCtx, &mut Scene);
pub type RoleFn<S> = dyn Fn(&S) -> Role;
pub type AccessFn<S> = dyn FnMut(&mut S, &mut AccessCtx, &mut NodeBuilder);
pub type ChildrenFn<S> = dyn Fn(&S) -> SmallVec<[WidgetId; 16]>;

#[cfg(FALSE)]
pub const REPLACE_CHILD: Selector = Selector::new("masonry-test.replace-child");

/// A widget that can be constructed from individual functions, builder-style.
///
/// This widget is generic over its state, which is passed in at construction time.
pub struct ModularWidget<S> {
    state: S,
    on_pointer_event: Option<Box<PointerEventFn<S>>>,
    on_text_event: Option<Box<TextEventFn<S>>>,
    on_access_event: Option<Box<AccessEventFn<S>>>,
    register_children: Option<Box<RegisterChildrenFn<S>>>,
    on_status_change: Option<Box<StatusChangeFn<S>>>,
    lifecycle: Option<Box<LifeCycleFn<S>>>,
    layout: Option<Box<LayoutFn<S>>>,
    compose: Option<Box<ComposeFn<S>>>,
    paint: Option<Box<PaintFn<S>>>,
    role: Option<Box<RoleFn<S>>>,
    access: Option<Box<AccessFn<S>>>,
    children: Option<Box<ChildrenFn<S>>>,
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
/// # use masonry::widget::Label;
/// # use masonry::{LifeCycle};
/// use masonry::testing::{Recording, Record, TestWidgetExt};
/// use masonry::testing::TestHarness;
/// use assert_matches::assert_matches;
/// let recording = Recording::default();
/// let widget = Label::new("Hello").record(&recording);
///
/// TestHarness::create(widget);
/// assert_matches!(recording.next().unwrap(), Record::RegisterChildren);
/// assert_matches!(recording.next().unwrap(), Record::L(LifeCycle::WidgetAdded));
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
    PE(PointerEvent),
    TE(TextEvent),
    AE(AccessEvent),
    RegisterChildren,
    SC(StatusChange),
    L(LifeCycle),
    Layout(Size),
    Compose,
    Paint,
    Access,
}

/// like `WidgetExt` but just for this one thing
pub trait TestWidgetExt: Widget + Sized + 'static {
    fn record(self, recording: &Recording) -> Recorder<Self> {
        Recorder {
            child: self,
            recording: recording.clone(),
        }
    }

    fn with_id(self, id: WidgetId) -> SizedBox {
        SizedBox::new_with_id(self, id)
    }
}

impl<W: Widget + 'static> TestWidgetExt for W {}

impl<S> ModularWidget<S> {
    pub fn new(state: S) -> Self {
        ModularWidget {
            state,
            on_pointer_event: None,
            on_text_event: None,
            on_access_event: None,
            register_children: None,
            on_status_change: None,
            lifecycle: None,
            layout: None,
            compose: None,
            paint: None,
            role: None,
            access: None,
            children: None,
        }
    }

    pub fn pointer_event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx, &PointerEvent) + 'static,
    ) -> Self {
        self.on_pointer_event = Some(Box::new(f));
        self
    }

    pub fn text_event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx, &TextEvent) + 'static,
    ) -> Self {
        self.on_text_event = Some(Box::new(f));
        self
    }

    pub fn access_event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx, &AccessEvent) + 'static,
    ) -> Self {
        self.on_access_event = Some(Box::new(f));
        self
    }

    pub fn register_children_fn(
        mut self,
        f: impl FnMut(&mut S, &mut RegisterCtx) + 'static,
    ) -> Self {
        self.register_children = Some(Box::new(f));
        self
    }

    pub fn status_change_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LifeCycleCtx, &StatusChange) + 'static,
    ) -> Self {
        self.on_status_change = Some(Box::new(f));
        self
    }

    pub fn lifecycle_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle) + 'static,
    ) -> Self {
        self.lifecycle = Some(Box::new(f));
        self
    }

    pub fn layout_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LayoutCtx, &BoxConstraints) -> Size + 'static,
    ) -> Self {
        self.layout = Some(Box::new(f));
        self
    }

    pub fn compose_fn(mut self, f: impl FnMut(&mut S, &mut ComposeCtx) + 'static) -> Self {
        self.compose = Some(Box::new(f));
        self
    }

    pub fn paint_fn(mut self, f: impl FnMut(&mut S, &mut PaintCtx, &mut Scene) + 'static) -> Self {
        self.paint = Some(Box::new(f));
        self
    }

    pub fn role_fn(mut self, f: impl Fn(&S) -> Role + 'static) -> Self {
        self.role = Some(Box::new(f));
        self
    }

    pub fn access_fn(
        mut self,
        f: impl FnMut(&mut S, &mut AccessCtx, &mut NodeBuilder) + 'static,
    ) -> Self {
        self.access = Some(Box::new(f));
        self
    }

    pub fn children_fn(
        mut self,
        children: impl Fn(&S) -> SmallVec<[WidgetId; 16]> + 'static,
    ) -> Self {
        self.children = Some(Box::new(children));
        self
    }
}

// TODO
// #[warn(clippy::missing_trait_methods)]
impl<S: 'static> Widget for ModularWidget<S> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &event::PointerEvent) {
        if let Some(f) = self.on_pointer_event.as_mut() {
            f(&mut self.state, ctx, event);
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &event::TextEvent) {
        if let Some(f) = self.on_text_event.as_mut() {
            f(&mut self.state, ctx, event);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        if let Some(f) = self.on_access_event.as_mut() {
            f(&mut self.state, ctx, event);
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        if let Some(f) = self.register_children.as_mut() {
            f(&mut self.state, ctx);
        }
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        if let Some(f) = self.on_status_change.as_mut() {
            f(&mut self.state, ctx, event);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        if let Some(f) = self.lifecycle.as_mut() {
            f(&mut self.state, ctx, event);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let ModularWidget {
            ref mut state,
            ref mut layout,
            ..
        } = self;
        layout
            .as_mut()
            .map(|f| f(state, ctx, bc))
            .unwrap_or_else(|| Size::new(100., 100.))
    }

    fn compose(&mut self, ctx: &mut ComposeCtx) {
        if let Some(f) = self.compose.as_mut() {
            f(&mut self.state, ctx);
        }
    }

    fn accessibility_role(&self) -> Role {
        if let Some(f) = self.role.as_ref() {
            f(&self.state)
        } else {
            Role::Unknown
        }
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        if let Some(f) = self.access.as_mut() {
            f(&mut self.state, ctx, node);
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        if let Some(f) = self.paint.as_mut() {
            f(&mut self.state, ctx, scene);
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        if let Some(f) = self.children.as_ref() {
            f(&self.state)
        } else {
            SmallVec::new()
        }
    }
}

impl ReplaceChild {
    pub fn new<W: Widget + 'static>(child: impl Widget, f: impl Fn() -> W + 'static) -> Self {
        let child = WidgetPod::new(child).boxed();
        let replacer = Box::new(move || WidgetPod::new(f()).boxed());
        ReplaceChild { child, replacer }
    }
}

impl Widget for ReplaceChild {
    #[cfg(FALSE)]
    fn on_event(&mut self, ctx: &mut EventCtx, event: &Event) {
        #[cfg(FALSE)]
        if let Event::Command(cmd) = event {
            if cmd.is(REPLACE_CHILD) {
                self.child = (self.replacer)();
                ctx.children_changed();
                return;
            }
        }
        self.child.on_event(ctx, event)
    }

    fn on_pointer_event(&mut self, _ctx: &mut EventCtx, _event: &event::PointerEvent) {}

    fn on_text_event(&mut self, _ctx: &mut EventCtx, _event: &event::TextEvent) {}

    fn on_access_event(&mut self, _ctx: &mut EventCtx, _event: &AccessEvent) {}

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, _event: &StatusChange) {
        ctx.request_layout();
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.child);
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle) {}

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        ctx.run_layout(&mut self.child, bc)
    }

    fn compose(&mut self, _ctx: &mut ComposeCtx) {}

    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {}

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        todo!()
    }
}

#[allow(dead_code)]
impl Recording {
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    /// Returns the next event in the recording, if one exists.
    ///
    /// This consumes the event.
    pub fn next(&self) -> Option<Record> {
        self.0.borrow_mut().pop_front()
    }

    /// Returns a vec of events drained from the recording.
    pub fn drain(&self) -> Vec<Record> {
        self.0.borrow_mut().drain(..).collect::<Vec<_>>()
    }

    fn push(&self, event: Record) {
        self.0.borrow_mut().push_back(event);
    }
}

impl<W: Widget> Widget for Recorder<W> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &event::PointerEvent) {
        self.recording.push(Record::PE(event.clone()));
        self.child.on_pointer_event(ctx, event);
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &event::TextEvent) {
        self.recording.push(Record::TE(event.clone()));
        self.child.on_text_event(ctx, event);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        self.recording.push(Record::AE(event.clone()));
        self.child.on_access_event(ctx, event);
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        self.recording.push(Record::RegisterChildren);
        self.child.register_children(ctx);
    }

    fn on_status_change(&mut self, ctx: &mut LifeCycleCtx, event: &StatusChange) {
        self.recording.push(Record::SC(event.clone()));
        self.child.on_status_change(ctx, event);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.recording.push(Record::L(event.clone()));
        self.child.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = self.child.layout(ctx, bc);
        self.recording.push(Record::Layout(size));
        size
    }

    fn compose(&mut self, ctx: &mut ComposeCtx) {
        self.recording.push(Record::Compose);
        self.child.compose(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.recording.push(Record::Paint);
        self.child.paint(ctx, scene);
    }

    fn accessibility_role(&self) -> Role {
        self.child.accessibility_role()
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx, node: &mut NodeBuilder) {
        self.recording.push(Record::Access);
        self.child.accessibility(ctx, node);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        self.child.children_ids()
    }
}
