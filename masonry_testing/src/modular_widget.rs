// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use tracing::trace_span;

use masonry_core::accesskit::{Node, Role};
use masonry_core::core::{
    AccessCtx, AccessEvent, BoxConstraints, ComposeCtx, EventCtx, LayoutCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetRef, find_widget_under_pointer,
};
use masonry_core::cursor_icon::CursorIcon;
use masonry_core::kurbo::{Point, Size};
use masonry_core::smallvec::SmallVec;
use masonry_core::vello::Scene;

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
pub(crate) type LayoutFn<S> =
    dyn FnMut(&mut S, &mut LayoutCtx<'_>, &mut PropertiesMut<'_>, &BoxConstraints) -> Size;
pub(crate) type ComposeFn<S> = dyn FnMut(&mut S, &mut ComposeCtx<'_>);
pub(crate) type PaintFn<S> = dyn FnMut(&mut S, &mut PaintCtx<'_>, &PropertiesRef<'_>, &mut Scene);
pub(crate) type RoleFn<S> = dyn Fn(&S) -> Role;
pub(crate) type AccessFn<S> = dyn FnMut(&mut S, &mut AccessCtx<'_>, &PropertiesRef<'_>, &mut Node);
pub(crate) type ChildrenFn<S> = dyn Fn(&S) -> SmallVec<[WidgetId; 16]>;

/// A widget that can be constructed from individual functions, builder-style.
///
/// This widget is generic over its state, which is passed in at construction time.
pub struct ModularWidget<S> {
    state: S,
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
    layout: Option<Box<LayoutFn<S>>>,
    compose: Option<Box<ComposeFn<S>>>,
    paint: Option<Box<PaintFn<S>>>,
    role: Option<Box<RoleFn<S>>>,
    access: Option<Box<AccessFn<S>>>,
    children: Option<Box<ChildrenFn<S>>>,
}

impl<S> ModularWidget<S> {
    /// Create a new `ModularWidget`.
    ///
    /// By default none of its methods do anything, and its layout method returns
    /// a static 100x100 size.
    pub fn new(state: S) -> Self {
        Self {
            state,
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
            layout: None,
            compose: None,
            paint: None,
            role: None,
            access: None,
            children: None,
        }
    }
}

/// Builder methods.
///
/// Each method takes a flag which is then returned by the matching Widget method.
impl<S> ModularWidget<S> {
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
/// Each method takes a callback that matches the behavior of the matching Widget method.
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

    /// See [`Widget::layout`]
    pub fn layout_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LayoutCtx<'_>, &mut PropertiesMut<'_>, &BoxConstraints) -> Size
        + 'static,
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
    pub fn children_fn(
        mut self,
        children: impl Fn(&S) -> SmallVec<[WidgetId; 16]> + 'static,
    ) -> Self {
        self.children = Some(Box::new(children));
        self
    }
}

#[warn(clippy::missing_trait_methods)]
impl<S: 'static> Widget for ModularWidget<S> {
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

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let Self { state, layout, .. } = self;
        layout
            .as_mut()
            .map(|f| f(state, ctx, props, bc))
            .unwrap_or_else(|| Size::new(100., 100.))
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

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        if let Some(f) = self.children.as_ref() {
            f(&self.state)
        } else {
            SmallVec::new()
        }
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
        CursorIcon::Default
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
}
