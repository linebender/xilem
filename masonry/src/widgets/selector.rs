// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::Vec2;

use crate::core::MeasureCtx;
use crate::core::keyboard::{Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayerType, LayoutCtx, NewWidget, PaintCtx,
    PointerButtonEvent, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Size};
use crate::layers::SelectorMenu;
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::theme;
use crate::util::debug_panic;
use crate::widgets::Label;
use crate::widgets::selector_item::SelectorItem;

/// A selector which displays a list of options when you click it.
///
/// This is called a "combo box" in some frameworks.
pub struct Selector {
    pub(crate) options: Vec<String>,
    pub(crate) selected_option: usize,
    pub(crate) child: WidgetPod<Label>,
    // TODO - Implement layer tracking in masonry_core instead.
    // Each widget should have access to a list of the layers they created.
    pub(crate) menu_layer_id: Option<WidgetId>,
}

// --- MARK: BUILDERS
impl Selector {
    /// Creates a new selector with a list of options.
    ///
    /// # Panics
    ///
    /// Panics when debug assertions are on if `options` is empty.
    pub fn new(mut options: Vec<String>) -> Self {
        if options.is_empty() {
            debug_panic!("cannot create selector with no option");
            options = vec![String::new()];
        }
        let first_option = options.first().unwrap().clone();

        Self {
            options,
            selected_option: 0,
            child: WidgetPod::new(Label::new(first_option)),
            menu_layer_id: None,
        }
    }

    /// Builder method to pre-set the widget's initial option.
    pub fn with_selected_option(mut self, mut selected_option: usize) -> Self {
        if selected_option >= self.options.len() {
            debug_panic!("invalid selector option index");
            selected_option = 0;
        }
        self.selected_option = selected_option;
        self
    }
}

// --- MARK: WIDGETMUT
impl Selector {
    /// Replaces the list of options with a new one.
    ///
    /// Selects the first option.
    ///
    /// # Panics
    ///
    /// Panics when debug assertions are on if `options` is empty.
    pub fn set_options(this: &mut WidgetMut<'_, Self>, mut options: Vec<String>) {
        if options.is_empty() {
            debug_panic!("cannot create selector with no option");
            options = vec![String::new()];
        }
        this.widget.options = options;
        Self::select_option(this, 0);
    }

    /// Selects the given option.
    ///
    /// # Panics
    ///
    /// Panics when debug assertions are on if `selected_option` is out of bounds.
    pub fn select_option(this: &mut WidgetMut<'_, Self>, mut selected_option: usize) {
        if selected_option >= this.widget.options.len() {
            debug_panic!("cannot select option {selected_option}: index out of bounds");
            selected_option = 0;
        }

        this.widget.selected_option = selected_option;

        let option = this.widget.options[selected_option].clone();
        Label::set_text(&mut Self::child_mut(this), option);
    }

    /// Returns a mutable reference to the child label.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

/// A [`Selector`]'s option was picked.
#[derive(PartialEq, Debug)]
pub struct SelectionChanged {
    /// The content of the picked selection.
    pub selected_content: String,
    /// The index of the picked selection.
    pub index: usize,
}

// --- MARK: HELPERS

impl Selector {
    fn toggle_selector_layer(&mut self, ctx: &mut EventCtx<'_>) {
        // If there's a selector menu, remove it.
        if let Some(id) = self.menu_layer_id {
            ctx.remove_layer(id);
            self.menu_layer_id = None;
            return;
        }

        // Else create selector menu

        let layer_type = LayerType::Selector {
            options: self.options.clone(),
            selected_option: self.selected_option,
        };

        let mut menu = SelectorMenu::new(ctx.widget_id());
        for option in self.options.iter() {
            let item = SelectorItem::new(option.clone());
            menu = menu.with(NewWidget::new(item));
        }

        let layer_widget = NewWidget::new(menu);

        // TODO: We should ideally create a layer with the same transform as this widget.
        ctx.create_layer(
            layer_type,
            layer_widget,
            ctx.window_origin() + Vec2::new(0., ctx.border_box_size().height),
        );
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Selector {
    type Action = SelectionChanged;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(..) => {
                if self.menu_layer_id.is_none() {
                    ctx.capture_pointer();
                }
            }
            PointerEvent::Up(PointerButtonEvent { .. }) => {
                if ctx.is_active() && ctx.is_hovered() {
                    self.toggle_selector_layer(ctx);
                }
            }
            _ => (),
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(event) if event.state.is_up() => {
                if matches!(&event.key, Key::Character(c) if c == " ")
                    || event.key == Key::Named(NamedKey::Enter)
                {
                    self.toggle_selector_layer(ctx);
                }
                // TODO - On arrow key, change selected_item
            }
            // TODO - Handle text selection
            // (e.g. if options are A, B and C, typing "C" should set selected_option to 2)
            // https://devblogs.microsoft.com/oldnewthing/20240408-00/?p=109627
            _ => (),
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match event.action {
            accesskit::Action::Click => {
                self.toggle_selector_layer(ctx);
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_)
            | Update::ActiveChanged(_)
            | Update::FocusChanged(_)
            | Update::DisabledChanged(_) => {
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);

        let child_length = ctx.compute_length(
            &mut self.child,
            auto_length,
            context_size,
            axis,
            cross_length,
        );

        let length = child_length;

        // TODO - Add MinimumSize property.
        // HACK: to make sure we look okay at default sizes when beside a text input,
        // we make sure we will have at least the same height as the default text input.
        // We also set a minimum width.
        match axis {
            Axis::Horizontal => length.max(theme::SELECTOR_MIN_WIDTH * scale),
            Axis::Vertical => length.max(theme::BORDERED_WIDGET_HEIGHT * scale),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.child, child_size);

        let child_origin = ((size - child_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.child, child_origin);

        let child_baseline = ctx.child_baseline_offset(&self.child);
        let child_bottom = child_origin.y + child_size.height;
        let bottom_gap = size.height - child_bottom;
        ctx.set_baseline_offset(child_baseline + bottom_gap);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}

    fn accessibility_role(&self) -> Role {
        Role::ComboBox
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.add_action(accesskit::Action::Click);
        // TODO - Add accesskit::Action::SetValue
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn accepts_focus(&self) -> bool {
        // Selectors can be tab-focused...
        true
    }

    fn accepts_text_input(&self) -> bool {
        // But they still aren't text areas.
        false
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Selector", id = id.trace())
    }
}
