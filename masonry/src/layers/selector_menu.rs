// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use masonry_core::core::{PointerButton, PointerButtonEvent, WidgetId};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::{Axis, Point, Size};

use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, CollectionWidget, ComposeCtx, EventCtx, HasProperty,
    Layer, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PointerEvent, PropertiesMut,
    PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use crate::layout::{LayoutSize, LenDef, LenReq, SizeDef};
use crate::properties::{Background, CornerRadius, Gap};
use crate::util::fill;
use crate::widgets::{SelectionChanged, Selector};

/// A [`Layer`] representing a list of options for a [`Selector`] widget.
pub struct SelectorMenu {
    creator: WidgetId,
    children: Vec<WidgetPod<dyn Widget>>,
}

// --- MARK: BUILDERS
impl SelectorMenu {
    /// Creates a new empty menu.
    pub fn new(creator: WidgetId) -> Self {
        Self {
            creator,
            children: Vec::new(),
        }
    }

    /// Builder-style method to add a child widget.
    pub fn with(mut self, child: NewWidget<impl Widget + ?Sized>) -> Self {
        let child = child.erased().to_pod();
        self.children.push(child);
        self
    }
}

// --- MARK: COLLECTIONWIDGET
impl CollectionWidget<()> for SelectorMenu {
    /// Returns the number of children.
    fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if there are no children.
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns a mutable reference to the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn get_mut<'t>(this: &'t mut WidgetMut<'_, Self>, idx: usize) -> WidgetMut<'t, dyn Widget> {
        let child = &mut this.widget.children[idx];
        this.ctx.get_mut(child)
    }

    /// Appends a child widget to the collection.
    fn add(
        this: &mut WidgetMut<'_, Self>,
        child: NewWidget<impl Widget + ?Sized>,
        _params: impl Into<()>,
    ) {
        let child = child.erased().to_pod();
        this.widget.children.push(child);
        this.ctx.children_changed();
    }

    /// Inserts a child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the number of children.
    fn insert(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        _params: impl Into<()>,
    ) {
        let child = child.erased().to_pod();
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Replaces the child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn set(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        child: NewWidget<impl Widget + ?Sized>,
        _params: impl Into<()>,
    ) {
        let child = child.erased().to_pod();
        let old_child = std::mem::replace(&mut this.widget.children[idx], child);
        this.ctx.remove_child(old_child);
    }

    /// Not applicable.
    fn set_params(_this: &mut WidgetMut<'_, Self>, _idx: usize, _params: impl Into<()>) {}

    /// Swaps the index of two children.
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    fn swap(this: &mut WidgetMut<'_, Self>, a: usize, b: usize) {
        this.widget.children.swap(a, b);
        this.ctx.children_changed();
    }

    /// Removes the child at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    fn remove(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child);
    }

    /// Removes all children.
    fn clear(this: &mut WidgetMut<'_, Self>) {
        for child in this.widget.children.drain(..) {
            this.ctx.remove_child(child);
        }
    }
}

// TODO - Add Border, Shadow and Padding properties

impl HasProperty<Background> for SelectorMenu {}
impl HasProperty<CornerRadius> for SelectorMenu {}
impl HasProperty<Gap> for SelectorMenu {}

impl Widget for SelectorMenu {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Up(PointerButtonEvent {
                button: None | Some(PointerButton::Primary),
                ..
            }) => {
                let self_id = ctx.widget_id();
                let clicked_id = ctx.target();
                let index = self
                    .children
                    .iter()
                    .position(|child| child.id() == clicked_id)
                    .unwrap();
                ctx.mutate_later(self.creator, move |mut selector| {
                    let mut selector = selector.downcast::<Selector>();
                    let selected_content = selector.widget.options[index].clone();

                    selector
                        .ctx
                        .submit_action::<SelectionChanged>(SelectionChanged {
                            selected_content,
                            index,
                        });
                    selector.ctx.remove_layer(self_id);
                    selector.widget.menu_layer_id = None;
                    Selector::select_option(&mut selector, index);
                });
            }
            _ => (),
        }
    }

    // TODO - Handle text selection
    // (e.g. if options are A, B and C, typing "C" should set selected option to 2)
    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::WindowFocusChange(false) => {
                ctx.remove_layer(ctx.widget_id());
            }
            _ => (),
        }
    }

    // TODO
    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
    }

    fn on_anim_frame(
        &mut self,
        _ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        // FIXME - This might be subject to TOCTOU. Find better system.
        if let Update::WidgetAdded = event {
            let id = ctx.widget_id();
            ctx.mutate_later(self.creator, move |mut selector| {
                let selector = selector.downcast::<Selector>();
                selector.widget.menu_layer_id = Some(id);
            });
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
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let gap = props.get::<Gap>();

        let gap_length = gap.gap.dp(scale);

        let (len_req, min_result) = match len_req {
            LenReq::MinContent | LenReq::MaxContent => (len_req, 0.),
            LenReq::FitContent(space) => (LenReq::MinContent, space),
        };

        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);

        let mut length: f64 = 0.;
        for child in &mut self.children {
            let child_length =
                ctx.compute_length(child, auto_length, context_size, axis, cross_length);
            match axis {
                Axis::Horizontal => length = length.max(child_length),
                Axis::Vertical => length += child_length,
            }
        }

        if axis == Axis::Vertical {
            let gap_count = (self.children.len() - 1) as f64;
            length += gap_count * gap_length;
        }

        min_result.max(length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let gap = props.get::<Gap>();

        let gap_count = (self.children.len() - 1) as f64;
        let gap_length = gap.gap.dp(scale);
        let total_child_vertical_space = size.height - gap_length * gap_count;
        let child_vertical_space = total_child_vertical_space / self.children.len() as f64;

        let width_def = LenDef::FitContent(size.width);
        let height_def = LenDef::FitContent(child_vertical_space.max(0.));
        let auto_size = SizeDef::new(width_def, height_def);
        let context_size = size.into();

        let mut y_offset = 0.0;
        for child in &mut self.children {
            let child_size = ctx.compute_size(child, auto_size, context_size);
            ctx.run_layout(child, child_size);
            ctx.place_child(child, Point::new(0.0, y_offset));

            y_offset += child_size.height + gap_length;
        }
    }

    fn compose(&mut self, _ctx: &mut ComposeCtx<'_>) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let border_radius = props.get::<CornerRadius>();
        let bg = props.get::<Background>();

        let bg_rect = ctx.size().to_rect().to_rounded_rect(border_radius.radius);

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in &mut self.children {
            ctx.register_child(child);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        self.children.iter().map(|child| child.id()).collect()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("SelectorMenu", id = id.trace())
    }
}

// --- MARK: IMPL LAYER
impl Layer for SelectorMenu {
    fn capture_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        let remove_this = match event {
            PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                let local_pos = ctx.local_position(state.position);

                !ctx.size().to_rect().contains(local_pos)
            }
            PointerEvent::Cancel(..) => true,
            _ => false,
        };

        if remove_this {
            ctx.remove_layer(ctx.widget_id());
            ctx.mutate_later(self.creator, move |mut selector| {
                let selector = selector.downcast::<Selector>();
                selector.widget.menu_layer_id = None;
            });
        }
    }
}
