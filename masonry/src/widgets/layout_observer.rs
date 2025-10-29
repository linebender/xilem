// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::mem;

use masonry_core::core::{
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, Widget, WidgetMut, WidgetPod,
};
use vello::kurbo::Size;

/// A widget which sends an [`LayoutChanged`] whenever its size changes.
///
/// Note: In theory, this probably shouldn't be a widget itself.
pub struct LayoutObserver {
    child: WidgetPod<dyn Widget>,
    size: Option<Size>,
}

// --- MARK: BUILDERS
impl LayoutObserver {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            size: None,
        }
    }
}

// --- MARK: WIDGETMUT
impl LayoutObserver {
    /// Give this container a child widget.
    ///
    /// The container's existing child will be overwritten.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        let old_child = mem::replace(&mut this.widget.child, child.erased().to_pod());
        this.ctx.remove_child(old_child);
        // Force a re-send when the layout changes.
        // This might be unnecessary, but it also shouldn't hurt.
        this.widget.size = None;
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    /// Force this layout observer to send a new action.
    ///
    /// It's hard to imagine reasonable use cases for this method, but it's provided for completeness.
    pub fn force_resend(this: &mut WidgetMut<'_, Self>) {
        this.widget.size = None;
        this.ctx.request_layout();
    }

    /// Get mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

/// The [action](Widget::Action) sent when the size of a widget has changed.
///
/// Currently only used by [`LayoutChanged`].
/// Note that this does not include the final size.
/// That should instead be accessed through [`MutateCtx::size`](crate::core::MutateCtx::size).
#[derive(Debug)]
pub struct LayoutChanged;

// --- MARK: IMPL WIDGET
impl Widget for LayoutObserver {
    type Action = LayoutChanged;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let res = ctx.run_layout(&mut self.child, bc);
        if self.size.is_none_or(|it| it != res) {
            ctx.submit_action::<Self::Action>(LayoutChanged);
        }
        self.size = Some(res);
        res
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _scene: &mut vello::Scene,
    ) {
    }

    fn accessibility_role(&self) -> accesskit::Role {
        accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
