// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use masonry_core::accesskit::{Node, Role};
use masonry_core::core::{
    AccessCtx, AccessEvent, ChildrenIds, ComposeCtx, CursorIcon, EventCtx, Layer, LayoutCtx,
    MeasureCtx, NewWidget, NoAction, PaintCtx, PointerEvent, Properties, PropertiesMut,
    PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetPod, WidgetRef, find_widget_under_pointer,
};
use masonry_core::kurbo::{Axis, Point, Size};
use masonry_core::layout::{LayoutSize, LenReq, SizeDef};
use masonry_core::vello::Scene;
use tracing::trace_span;

pub(crate) type PointerEventFn<S> =
    dyn FnMut(&mut S, &mut EventCtx<'_>, &mut PropertiesMut<'_>, &PointerEvent);
pub(crate) type TextEventFn<S> =
    dyn FnMut(&mut S, &mut EventCtx<'_>, &mut PropertiesMut<'_>, &TextEvent);
pub(crate) type AccessEventFn<S> =
    dyn FnMut(&mut S, &mut EventCtx<'_>, &mut PropertiesMut<'_>, &AccessEvent);
pub(crate) type AnimFrameFn<S> = dyn FnMut(&mut S, &mut UpdateCtx<'_>, &mut PropertiesMut<'_>, u64);
pub(crate) type RegisterChildrenFn<S> = dyn FnMut(&mut S, &mut RegisterCtx<'_>);
pub(crate) type UpdateFn<S> =
    dyn FnMut(&mut S, &mut UpdateCtx<'_>, &mut PropertiesMut<'_>, &Update);
pub(crate) type PropertyChangeFn<S> = dyn FnMut(&mut S, &mut UpdateCtx<'_>, TypeId);
pub(crate) type MeasureFn<S> =
    dyn FnMut(&mut S, &mut MeasureCtx<'_>, &PropertiesRef<'_>, Axis, LenReq, Option<f64>) -> f64;
pub(crate) type LayoutFn<S> = dyn FnMut(&mut S, &mut LayoutCtx<'_>, &PropertiesRef<'_>, Size);
pub(crate) type ComposeFn<S> = dyn FnMut(&mut S, &mut ComposeCtx<'_>);
pub(crate) type PaintFn<S> = dyn FnMut(&mut S, &mut PaintCtx<'_>, &PropertiesRef<'_>, &mut Scene);
pub(crate) type RoleFn<S> = dyn Fn(&S) -> Role;
pub(crate) type AccessFn<S> = dyn FnMut(&mut S, &mut AccessCtx<'_>, &PropertiesRef<'_>, &mut Node);
pub(crate) type ChildrenFn<S> = dyn Fn(&S) -> ChildrenIds;

/// A widget that can be constructed from individual functions, builder-style.
///
/// This widget is generic over its state, which is passed in at construction time.
pub struct ModularWidget<S> {
    /// The state passed to all the callbacks of this widget
    pub state: S,
    icon: CursorIcon,
    accepts_pointer_interaction: bool,
    accepts_focus: bool,
    accepts_text_input: bool,
    on_pointer_event: Option<Box<PointerEventFn<S>>>,
    on_text_event: Option<Box<TextEventFn<S>>>,
    on_access_event: Option<Box<AccessEventFn<S>>>,
    on_anim_frame: Option<Box<AnimFrameFn<S>>>,
    register_children: Option<Box<RegisterChildrenFn<S>>>,
    update: Option<Box<UpdateFn<S>>>,
    property_change: Option<Box<PropertyChangeFn<S>>>,
    measure: Option<Box<MeasureFn<S>>>,
    layout: Option<Box<LayoutFn<S>>>,
    compose: Option<Box<ComposeFn<S>>>,
    paint: Option<Box<PaintFn<S>>>,
    post_paint: Option<Box<PaintFn<S>>>,
    role: Option<Box<RoleFn<S>>>,
    access: Option<Box<AccessFn<S>>>,
    children: Option<Box<ChildrenFn<S>>>,
}

impl<S> ModularWidget<S> {
    /// Creates a new `ModularWidget`.
    ///
    /// By default none of its methods do anything, and its layout method returns
    /// a static 100x100 size.
    pub fn new(state: S) -> Self {
        Self {
            state,
            icon: CursorIcon::Default,
            accepts_pointer_interaction: true,
            accepts_focus: false,
            accepts_text_input: false,
            on_pointer_event: None,
            on_text_event: None,
            on_access_event: None,
            on_anim_frame: None,
            register_children: None,
            update: None,
            property_change: None,
            measure: None,
            layout: None,
            compose: None,
            paint: None,
            post_paint: None,
            role: None,
            access: None,
            children: None,
        }
    }
}

impl<W: Widget + ?Sized> ModularWidget<WidgetPod<W>> {
    /// Creates a new `ModularWidget` with some methods already set to handle a single child.
    pub fn new_parent(child: NewWidget<W>) -> Self {
        let child = child.to_pod();
        Self::new(child)
            .register_children_fn(move |child, ctx| {
                ctx.register_child(child);
            })
            .measure_fn(move |child, ctx, _props, axis, len_req, cross_length| {
                let auto_length = len_req.into();
                let context_size = LayoutSize::maybe(axis.cross(), cross_length);

                ctx.compute_length(child, auto_length, context_size, axis, cross_length)
            })
            .layout_fn(move |child, ctx, _props, size| {
                let child_size = ctx.compute_size(child, SizeDef::fit(size), size.into());
                ctx.run_layout(child, child_size);
                ctx.place_child(child, Point::ZERO);
            })
            .children_fn(|child| ChildrenIds::from_slice(&[child.id()]))
    }
}

impl<W: Widget + ?Sized> ModularWidget<Vec<WidgetPod<W>>> {
    /// Creates a new `ModularWidget` with some methods already set to handle multiple children.
    ///
    /// Layout will just stack all children on the same position and return the size of the largest.
    pub fn new_multi_parent(children: Vec<NewWidget<W>>) -> Self {
        let children = children.into_iter().map(|child| child.to_pod()).collect();
        Self::new(children)
            .register_children_fn(move |children, ctx| {
                for child in children {
                    ctx.register_child(child);
                }
            })
            .measure_fn(move |children, ctx, _props, axis, len_req, cross_length| {
                let auto_length = len_req.into();
                let context_size = LayoutSize::maybe(axis.cross(), cross_length);

                let mut length: f64 = 0.;
                for child in children {
                    let child_length =
                        ctx.compute_length(child, auto_length, context_size, axis, cross_length);
                    length = length.max(child_length);
                }

                length
            })
            .layout_fn(move |children, ctx, _props, size| {
                let auto_size = SizeDef::fit(size);
                let context_size = size.into();

                for child in children {
                    let child_size = ctx.compute_size(child, auto_size, context_size);
                    ctx.run_layout(child, child_size);
                    ctx.place_child(child, Point::ZERO);
                }
            })
            .children_fn(|children| children.iter().map(|child| child.id()).collect())
    }
}

/// Builder methods.
///
/// Each method takes a flag which is then returned by the matching [`Widget`] method.
impl<S> ModularWidget<S> {
    /// See [`Widget::get_cursor`]
    pub fn cursor_icon(mut self, icon: CursorIcon) -> Self {
        self.icon = icon;
        self
    }

    /// See [`Widget::accepts_pointer_interaction`]
    pub fn accepts_pointer_interaction(mut self, flag: bool) -> Self {
        self.accepts_pointer_interaction = flag;
        self
    }

    /// See [`Widget::accepts_focus`]
    pub fn accepts_focus(mut self, flag: bool) -> Self {
        self.accepts_focus = flag;
        self
    }

    /// See [`Widget::accepts_text_input`]
    pub fn accepts_text_input(mut self, flag: bool) -> Self {
        self.accepts_text_input = flag;
        self
    }
}

/// Builder methods.
///
/// Each method takes a callback that matches the behavior of the matching [`Widget`] method.
impl<S> ModularWidget<S> {
    /// See [`Widget::on_pointer_event`]
    pub fn pointer_event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx<'_>, &mut PropertiesMut<'_>, &PointerEvent) + 'static,
    ) -> Self {
        self.on_pointer_event = Some(Box::new(f));
        self
    }

    /// See [`Widget::on_text_event`]
    pub fn text_event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx<'_>, &mut PropertiesMut<'_>, &TextEvent) + 'static,
    ) -> Self {
        self.on_text_event = Some(Box::new(f));
        self
    }

    /// See [`Widget::on_access_event`]
    pub fn access_event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx<'_>, &mut PropertiesMut<'_>, &AccessEvent) + 'static,
    ) -> Self {
        self.on_access_event = Some(Box::new(f));
        self
    }

    /// See [`Widget::on_anim_frame`]
    pub fn anim_frame_fn(
        mut self,
        f: impl FnMut(&mut S, &mut UpdateCtx<'_>, &mut PropertiesMut<'_>, u64) + 'static,
    ) -> Self {
        self.on_anim_frame = Some(Box::new(f));
        self
    }

    /// See [`Widget::register_children`]
    pub fn register_children_fn(
        mut self,
        f: impl FnMut(&mut S, &mut RegisterCtx<'_>) + 'static,
    ) -> Self {
        self.register_children = Some(Box::new(f));
        self
    }

    /// See [`Widget::update`]
    pub fn update_fn(
        mut self,
        f: impl FnMut(&mut S, &mut UpdateCtx<'_>, &mut PropertiesMut<'_>, &Update) + 'static,
    ) -> Self {
        self.update = Some(Box::new(f));
        self
    }

    /// See [`Widget::property_changed`]
    pub fn property_change_fn(
        mut self,
        f: impl FnMut(&mut S, &mut UpdateCtx<'_>, TypeId) + 'static,
    ) -> Self {
        self.property_change = Some(Box::new(f));
        self
    }

    /// See [`Widget::measure`]
    pub fn measure_fn(
        mut self,
        f: impl FnMut(&mut S, &mut MeasureCtx<'_>, &PropertiesRef<'_>, Axis, LenReq, Option<f64>) -> f64
        + 'static,
    ) -> Self {
        self.measure = Some(Box::new(f));
        self
    }

    /// See [`Widget::layout`]
    pub fn layout_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LayoutCtx<'_>, &PropertiesRef<'_>, Size) + 'static,
    ) -> Self {
        self.layout = Some(Box::new(f));
        self
    }

    /// See [`Widget::compose`]
    pub fn compose_fn(mut self, f: impl FnMut(&mut S, &mut ComposeCtx<'_>) + 'static) -> Self {
        self.compose = Some(Box::new(f));
        self
    }

    /// See [`Widget::paint`]
    pub fn paint_fn(
        mut self,
        f: impl FnMut(&mut S, &mut PaintCtx<'_>, &PropertiesRef<'_>, &mut Scene) + 'static,
    ) -> Self {
        self.paint = Some(Box::new(f));
        self
    }

    /// See [`Widget::post_paint`]
    pub fn post_paint_fn(
        mut self,
        f: impl FnMut(&mut S, &mut PaintCtx<'_>, &PropertiesRef<'_>, &mut Scene) + 'static,
    ) -> Self {
        self.post_paint = Some(Box::new(f));
        self
    }

    /// See [`Widget::accessibility_role`]
    pub fn role_fn(mut self, f: impl Fn(&S) -> Role + 'static) -> Self {
        self.role = Some(Box::new(f));
        self
    }

    /// See [`Widget::accessibility`]
    pub fn access_fn(
        mut self,
        f: impl FnMut(&mut S, &mut AccessCtx<'_>, &PropertiesRef<'_>, &mut Node) + 'static,
    ) -> Self {
        self.access = Some(Box::new(f));
        self
    }

    /// See [`Widget::children_ids`]
    pub fn children_fn(mut self, children: impl Fn(&S) -> ChildrenIds + 'static) -> Self {
        self.children = Some(Box::new(children));
        self
    }
}

#[warn(clippy::missing_trait_methods)]
impl<S: 'static> Widget for ModularWidget<S> {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if let Some(f) = self.on_pointer_event.as_mut() {
            f(&mut self.state, ctx, props, event);
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if let Some(f) = self.on_text_event.as_mut() {
            f(&mut self.state, ctx, props, event);
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if let Some(f) = self.on_access_event.as_mut() {
            f(&mut self.state, ctx, props, event);
        }
    }

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
        if let Some(f) = self.on_anim_frame.as_mut() {
            f(&mut self.state, ctx, props, interval);
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(f) = self.register_children.as_mut() {
            f(&mut self.state, ctx);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, event: &Update) {
        if let Some(f) = self.update.as_mut() {
            f(&mut self.state, ctx, props, event);
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if let Some(f) = self.property_change.as_mut() {
            f(&mut self.state, ctx, property_type);
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let Self { state, measure, .. } = self;
        measure
            .as_mut()
            .map(|f| f(state, ctx, props, axis, len_req, cross_length))
            .unwrap_or_default()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        if let Some(f) = self.layout.as_mut() {
            f(&mut self.state, ctx, props, size);
        }
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
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

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if let Some(f) = self.access.as_mut() {
            f(&mut self.state, ctx, props, node);
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        if let Some(f) = self.paint.as_mut() {
            f(&mut self.state, ctx, props, scene);
        }
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        if let Some(f) = self.post_paint.as_mut() {
            f(&mut self.state, ctx, props, scene);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        if let Some(f) = self.children.as_ref() {
            f(&self.state)
        } else {
            ChildrenIds::new()
        }
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        None
    }

    fn accepts_pointer_interaction(&self) -> bool {
        self.accepts_pointer_interaction
    }

    fn accepts_focus(&self) -> bool {
        self.accepts_focus
    }

    fn accepts_text_input(&self) -> bool {
        self.accepts_text_input
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("ModularWidget", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        None
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        self.icon
    }

    fn find_widget_under_pointer<'c>(
        &'c self,
        ctx: QueryCtx<'c>,
        pos: Point,
    ) -> Option<WidgetRef<'c, dyn Widget>> {
        find_widget_under_pointer(self, ctx, pos)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn short_type_name(&self) -> &'static str {
        "ModularWidget"
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
