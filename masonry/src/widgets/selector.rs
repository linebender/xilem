// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use accesskit::{Node, Role};
use tracing::{Span, trace_span};

use crate::core::keyboard::{Key, NamedKey};
use crate::core::{
    AccessCtx, AccessEvent, AnchorPlacement, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx,
    NewWidget, PaintCtx, PointerButtonEvent, PointerEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, SubwindowPlacement, SubwindowType, TextEvent, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod,
};
use crate::imaging::Painter;
use crate::kurbo::{Axis, Size};
use crate::layout::{AsUnit, LayoutSize, LenReq, Length, SizeDef};
use crate::subwindows::SelectorMenu;
use crate::theme;
use crate::util::debug_panic;
use crate::widgets::{Label, SelectorItem};

/// A selector which displays a list of options when you click it.
///
/// This is called a "combo box" in some frameworks.
pub struct Selector {
    pub(crate) options: Vec<String>,
    pub(crate) selected_option: usize,
    pub(crate) child: WidgetPod<Label>,
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

        Self {
            options,
            selected_option: 0,
            child: WidgetPod::new(Label::new("")),
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
    fn toggle_selector_subwindow(&mut self, ctx: &mut EventCtx<'_>) {
        // If there's a selector menu, remove it.
        if let Some(menu) = ctx.get_attached_subwindow::<SelectorMenu>() {
            ctx.remove_subwindow(menu);
            return;
        }

        // Else create selector menu

        let subwindow_type = SubwindowType::Selector {
            options: self.options.clone(),
            selected_option: self.selected_option,
        };

        let mut menu = SelectorMenu::new(ctx.widget_id());
        for option in self.options.iter() {
            let item = SelectorItem::new(option.clone());
            menu = menu.with(NewWidget::new(item));
        }

        // TODO: We should ideally create a subwindow with the same transform as this widget.
        let anchor = ctx.window_transform().transform_rect_bbox(ctx.border_box());
        ctx.create_attached_subwindow(
            subwindow_type,
            NewWidget::new(menu),
            SubwindowPlacement::Anchored {
                anchor,
                preferred: AnchorPlacement::BOTTOM_START,
                gap: 0.0,
            },
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
            PointerEvent::Down(..) if ctx.get_attached_subwindow::<SelectorMenu>().is_none() => {
                ctx.capture_pointer();
            }
            PointerEvent::Up(PointerButtonEvent { .. }) if ctx.is_active() && ctx.is_hovered() => {
                self.toggle_selector_subwindow(ctx);
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
            TextEvent::Keyboard(event)
                if event.state.is_up()
                    && (matches!(&event.key, Key::Character(c) if c == " ")
                        || event.key == Key::Named(NamedKey::Enter)) =>
            {
                self.toggle_selector_subwindow(ctx);
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
                self.toggle_selector_subwindow(ctx);
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                let option = self.options[self.selected_option].clone();
                ctx.mutate_self_later(move |mut selector| {
                    let mut selector = selector.downcast::<Self>();
                    let mut label = Self::child_mut(&mut selector);
                    Label::set_text(&mut label, option);
                });
            }
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
        cross_length: Option<Length>,
    ) -> Length {
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
            Axis::Horizontal => length.max(theme::SELECTOR_MIN_WIDTH.px()),
            Axis::Vertical => length.max(theme::BORDERED_WIDGET_HEIGHT.px()),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.child, child_size);

        let child_origin = ((size - child_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.child, child_origin);

        ctx.derive_baselines(&self.child);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
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

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PointerButton;
    use crate::layout::UnitPoint;
    use crate::testing::TestHarness;
    use crate::theme::test_property_set;
    use crate::widgets::Align;

    #[test]
    fn with_selected_option() {
        let options = vec!["Blue and black".into(), "White and gold".into()];
        let selector = Selector::new(options).with_selected_option(1);
        let mut harness = TestHarness::create_with_size(
            test_property_set(),
            NewWidget::new(selector),
            (200_u32, 60_u32),
        );

        harness.edit_root_widget(|mut selector| {
            let label = Selector::child_mut(&mut selector);
            assert_eq!(&**label.widget.text(), "White and gold");
        });
    }

    #[test]
    fn menu_opens_below_or_flips_above_the_selector() {
        for (alignment, opens_below) in [(UnitPoint::TOP, true), (UnitPoint::BOTTOM, false)] {
            let selector = Selector::new(vec!["Short".into(), "A wider option".into()]).prepare();
            let id = selector.id();
            let root = Align::new(alignment, selector).prepare();
            let mut harness = TestHarness::create_with_size(test_property_set(), root, (300, 200));
            harness.mouse_click_on(id, Some(PointerButton::Primary));

            let root = harness.render_root();
            let selector = root.get_widget(id).unwrap();
            let menu = root.get_subwindow_root(1);
            let selector_ctx = selector.ctx();
            let menu_ctx = menu.ctx();
            let selector = selector_ctx
                .window_transform()
                .transform_rect_bbox(selector_ctx.border_box());
            let menu = menu_ctx
                .window_transform()
                .transform_rect_bbox(menu_ctx.border_box());
            assert_eq!(menu.x0, selector.x0);
            if opens_below {
                assert_eq!(menu.y0, selector.y1);
            } else {
                assert_eq!(menu.y1, selector.y0);
            }
        }
    }

    // TODO - Add holistic tests that exercise subwindow behavior.
}
