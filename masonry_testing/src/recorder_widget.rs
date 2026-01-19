// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper widgets for writing tests.
//!
//! This includes tools for making throwaway widgets more easily.
//!
//! Note: Some of these types are undocumented. They're meant to help maintainers of
//! Masonry, not to be user-facing.

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use masonry_core::accesskit::{Node, Role};
use masonry_core::core::{
    AccessCtx, AccessEvent, ChildrenIds, ComposeCtx, CursorIcon, EventCtx, Layer, LayoutCtx,
    MeasureCtx, NewWidget, PaintCtx, PointerEvent, Properties, PropertiesMut, PropertiesRef,
    QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetRef,
};
use masonry_core::kurbo::{Axis, Point, Size};
use masonry_core::layout::LenReq;
use masonry_core::vello::Scene;

// TODO - Re-enable doc test.
// Doc test is currently disabled because it depends on a parent crate.

/// A wrapper widget that records each time one of its methods is called.
///
/// Its intent is to let you observe the methods called on a widget in a test.
///
/// Make one like this:
///
/// ```rust,ignore
/// # use masonry::widgets::Label;
/// # use masonry::core::Update;
/// use masonry::testing::{TestHarness, Recording, Record, TestWidgetExt};
/// use assert_matches::assert_matches;
///
/// let recording = Recording::default();
/// let widget = Label::new("Hello").record(&recording);
///
/// TestHarness::create(DefaultProperties::default(), widget);
/// assert_matches!(recording.next().unwrap(), Record::RC);
/// assert_matches!(recording.next().unwrap(), Record::U(Update::WidgetAdded));
/// ```
pub struct Recorder<W: Widget> {
    recording: Recording,
    child: W,
}

// TODO - Remove interior mutability, which is no longer needed.
// Store a VecDequeue directly.
/// A recording of widget method calls.
///
/// Internally stores a queue of [`Records`](Record).
#[derive(Debug, Clone, Default)]
pub struct Recording(Rc<RefCell<VecDeque<Record>>>);

/// A recording of a method call on a widget.
///
/// Each member of the enum corresponds to one of the [`Widget`] trait methods.
#[derive(Debug, Clone)]
pub enum Record {
    /// Pointer event.
    PointerEvent(PointerEvent),
    /// Text event.
    TextEvent(TextEvent),
    /// Access event.
    AccessEvent(AccessEvent),
    /// Animation frame.
    AnimFrame(u64),
    /// Register children
    RegisterChildren,
    /// Update
    Update(Update),
    /// Property change.
    PropertyChange(TypeId),
    /// Measure. Records the length returned by the measure method.
    Measure(f64),
    /// Layout. Records the size given to the layout method.
    Layout(Size),
    /// Compose.
    Compose,
    /// Paint.
    Paint,
    /// Paint after children.
    PostPaint,
    /// Accessibility.
    Accessibility,
}

impl Recording {
    /// True if no events have been recorded.
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    /// The number of events in the recording.
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    /// Clear recorded events.
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

impl<W: Widget> Recorder<W> {
    /// Wraps child widget in a Recorder that records all method calls.
    pub fn new(child: W, recording: &Recording) -> Self {
        Self {
            child,
            recording: recording.clone(),
        }
    }

    /// Returns the events this widget recorded.
    pub fn recording(&self) -> &Recording {
        &self.recording
    }

    /// Returns a reference to the child that's being recorded.
    pub fn inner(&self) -> &W {
        &self.child
    }

    /// Returns an exclusive reference to the child that's being recorded.
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.child
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W: Widget> Widget for Recorder<W> {
    type Action = W::Action;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        self.recording.push(Record::PointerEvent(event.clone()));
        self.child.on_pointer_event(ctx, props, event);
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        self.recording.push(Record::TextEvent(event.clone()));
        self.child.on_text_event(ctx, props, event);
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        self.recording.push(Record::AccessEvent(event.clone()));
        self.child.on_access_event(ctx, props, event);
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
        self.recording.push(Record::AnimFrame(interval));
        self.child.on_anim_frame(ctx, props, interval);
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        self.recording.push(Record::RegisterChildren);
        self.child.register_children(ctx);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, event: &Update) {
        self.recording.push(Record::Update(event.clone()));
        self.child.update(ctx, props, event);
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        self.recording.push(Record::PropertyChange(property_type));
        self.child.property_changed(ctx, property_type);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let length = self.child.measure(ctx, props, axis, len_req, cross_length);
        self.recording.push(Record::Measure(length));
        length
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        self.recording.push(Record::Layout(size));
        self.child.layout(ctx, props, size);
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        self.recording.push(Record::Compose);
        self.child.compose(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        self.recording.push(Record::Paint);
        self.child.paint(ctx, props, scene);
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        self.recording.push(Record::PostPaint);
        self.child.post_paint(ctx, props, scene);
    }

    fn accessibility_role(&self) -> Role {
        self.child.accessibility_role()
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        self.recording.push(Record::Accessibility);
        self.child.accessibility(ctx, props, node);
    }

    fn children_ids(&self) -> ChildrenIds {
        self.child.children_ids()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        None
    }

    fn accepts_pointer_interaction(&self) -> bool {
        self.child.accepts_pointer_interaction()
    }

    fn accepts_focus(&self) -> bool {
        self.child.accepts_focus()
    }

    fn accepts_text_input(&self) -> bool {
        self.child.accepts_text_input()
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        self.child.make_trace_span(id)
    }

    fn get_debug_text(&self) -> Option<String> {
        self.child.get_debug_text()
    }

    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        self.child.get_cursor(ctx, pos)
    }

    fn find_widget_under_pointer<'c>(
        &'c self,
        ctx: QueryCtx<'c>,
        pos: Point,
    ) -> Option<WidgetRef<'c, dyn Widget>> {
        self.child.find_widget_under_pointer(ctx, pos)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn short_type_name(&self) -> &'static str {
        "Recorder"
    }

    fn with_auto_id(self) -> NewWidget<Self>
    where
        Self: Sized,
    {
        NewWidget::new(self)
    }

    fn with_props(self, props: impl Into<Properties>) -> NewWidget<Self>
    where
        Self: Sized,
    {
        NewWidget::new_with_props(self, props)
    }
}
