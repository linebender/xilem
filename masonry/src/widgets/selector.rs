// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A selector widget.

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};
use vello::Scene;
use vello::kurbo::Vec2;

use crate::core::keyboard::{Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayerType, LayoutCtx, NewWidget, PaintCtx,
    PointerButtonEvent, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::core::{HasProperty, MeasureCtx};
use crate::kurbo::{Affine, Axis, Size};
use crate::layers::SelectorMenu;
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::properties::{
    ActiveBackground, Background, BorderColor, BorderWidth, BoxShadow, CornerRadius,
    DisabledBackground, FocusedBorderColor, HoveredBorderColor, Padding,
};
use crate::theme;
use crate::util::{debug_panic, fill, stroke};
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
    pub fn with_selected_option(mut self, selected_option: usize) -> Self {
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

impl HasProperty<DisabledBackground> for Selector {}
impl HasProperty<ActiveBackground> for Selector {}
impl HasProperty<Background> for Selector {}
impl HasProperty<FocusedBorderColor> for Selector {}
impl HasProperty<HoveredBorderColor> for Selector {}
impl HasProperty<BorderColor> for Selector {}
impl HasProperty<BorderWidth> for Selector {}
impl HasProperty<CornerRadius> for Selector {}
impl HasProperty<Padding> for Selector {}

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

        ctx.create_layer(
            layer_type,
            layer_widget,
            ctx.window_origin() + Vec2::new(0., ctx.size().height),
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

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        DisabledBackground::prop_changed(ctx, property_type);
        ActiveBackground::prop_changed(ctx, property_type);
        Background::prop_changed(ctx, property_type);
        FocusedBorderColor::prop_changed(ctx, property_type);
        HoveredBorderColor::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
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

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let border_length = border.length(axis).dp(scale);
        let padding_length = padding.length(axis).dp(scale);

        let cross = axis.cross();
        let cross_space = cross_length.map(|cross_length| {
            let cross_border_length = border.length(cross).dp(scale);
            let cross_padding_length = padding.length(cross).dp(scale);
            (cross_length - cross_border_length - cross_padding_length).max(0.)
        });

        let auto_length = len_req.reduce(border_length + padding_length).into();
        let context_size = LayoutSize::maybe(cross, cross_space);

        let child_length = ctx.compute_length(
            &mut self.child,
            auto_length,
            context_size,
            axis,
            cross_space,
        );

        let length = child_length + border_length + padding_length;

        // TODO - Add MinimumSize property.
        // HACK: to make sure we look okay at default sizes when beside a text input,
        // we make sure we will have at least the same height as the default text input.
        // We also set a minimum width.
        match axis {
            Axis::Horizontal => length.max(theme::SELECTOR_MIN_WIDTH * scale),
            Axis::Vertical => length.max(theme::BORDERED_WIDGET_HEIGHT * scale),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        // TODO: Remove HACK: Until scale factor rework happens, just pretend it's always 1.0.
        //       https://github.com/linebender/xilem/issues/1264
        let scale = 1.0;

        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();
        let shadow = props.get::<BoxShadow>();

        let space = border.size_down(size, scale);
        let space = padding.size_down(space, scale);

        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(space), space.into());
        ctx.run_layout(&mut self.child, child_size);

        let child_origin = ((size - child_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.child, child_origin);

        let baseline = ctx.child_baseline_offset(&self.child);
        let baseline = border.baseline_up(baseline, scale);
        let baseline = padding.baseline_up(baseline, scale);
        ctx.set_baseline_offset(baseline);

        if shadow.is_visible() {
            ctx.set_paint_insets(shadow.get_insets());
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let is_focused = ctx.is_focus_target();
        let is_pressed = ctx.is_active();
        let is_hovered = ctx.is_hovered();
        let size = ctx.size();

        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();

        let bg = if ctx.is_disabled() {
            &props.get::<DisabledBackground>().0
        } else if is_pressed {
            &props.get::<ActiveBackground>().0
        } else {
            props.get::<Background>()
        };

        let bg_rect = border_width.bg_rect(size, border_radius);
        let border_rect = border_width.border_rect(size, border_radius);

        let border_color = if is_focused {
            &props.get::<FocusedBorderColor>().0
        } else if is_hovered {
            &props.get::<HoveredBorderColor>().0
        } else {
            props.get::<BorderColor>()
        };

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let border_radius = props.get::<CornerRadius>();
        let shadow = props.get::<BoxShadow>();

        let shadow_rect = shadow.shadow_rect(size, border_radius);

        shadow.paint(scene, Affine::IDENTITY, shadow_rect);
    }

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
