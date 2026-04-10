// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use accesskit::{Node, Role};
use tracing::{Span, trace_span};

use crate::core::{
    AccessCtx, ActionCtx, ChildrenIds, ErasedAction, FromDynWidget, LayoutCtx, MeasureCtx,
    MutateCtx, NewWidget, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx,
    Widget, WidgetId, WidgetMut, WidgetPod,
};
use crate::imaging::Painter;
use crate::kurbo::{Axis, Point, Size};
use crate::layout::{LayoutSize, LenReq, SizeDef};
use crate::widgets::{Button, ButtonPress, Label};

/// Pagination for navigating between different page numbers.
pub struct Pagination {
    /// Total number of pages.
    page_count: usize,
    /// The 0-based index of the currently active page.
    active_page: usize,

    /// The maximum number of buttons shown for the first pages.
    buttons_start: u8,
    /// The maximum number of buttons shown for the last pages.
    buttons_end: u8,
    /// The maximum number of buttons shown in total.
    buttons_total: u8,

    /// All the page buttons.
    buttons: Vec<PageButton>,
}

/// One button for a specific page.
struct PageButton {
    widget: WidgetPod<Button>,
    disabled: bool,
    stashed: bool,
    page_idx: usize,
}

/// --- MARK: BUILDERS
impl Pagination {
    /// Creates a new [`Pagination`] with the given `page_count`.
    pub fn new(page_count: usize) -> Self {
        Self {
            page_count,
            active_page: 0,
            buttons_start: 1,
            buttons_end: 1,
            buttons_total: 9,
            buttons: Vec::new(),
        }
    }

    /// Sets the currently active page.
    ///
    /// This is a 0-based index and defaults to `0`.
    ///
    /// It is clamped to the total page count.
    pub fn active_page(mut self, active_page: usize) -> Self {
        self.active_page = if self.page_count == 0 {
            0
        } else {
            active_page.min(self.page_count - 1)
        };
        self
    }

    /// Sets the maximum number of buttons to always show for the first pages.
    ///
    /// This defaults to `1`.
    pub fn buttons_start(mut self, buttons_start: u8) -> Self {
        self.buttons_start = buttons_start;
        self
    }

    /// Sets the maximum number of buttons to always show for the last pages.
    ///
    /// This defaults to `1`.
    pub fn buttons_end(mut self, buttons_end: u8) -> Self {
        self.buttons_end = buttons_end;
        self
    }

    /// Sets the maximum number of buttons shown in total.
    ///
    /// The effective button limit also depends on the total page count.
    ///
    /// This defaults to `9`.
    pub fn buttons_total(mut self, buttons_total: u8) -> Self {
        self.buttons_total = buttons_total;
        self
    }
}

// --- MARK: WIDGETMUT
impl Pagination {
    /// Sets the total page count.
    ///
    /// The active page index is clamped to this new total.
    pub fn set_page_count(this: &mut WidgetMut<'_, Self>, page_count: usize) {
        this.widget.page_count = page_count;
        this.widget.active_page = if page_count == 0 {
            0
        } else {
            this.widget.active_page.min(page_count - 1)
        };

        this.widget.sync_buttons(&mut this.ctx);
    }

    /// Sets the currently active page.
    ///
    /// This is a 0-based index.
    ///
    /// It is clamped to the total page count.
    pub fn set_active_page(this: &mut WidgetMut<'_, Self>, active_page: usize) {
        this.widget.active_page = if this.widget.page_count == 0 {
            0
        } else {
            active_page.min(this.widget.page_count - 1)
        };

        this.widget.sync_buttons(&mut this.ctx);
    }

    /// Sets the maximum number of buttons to always show for the first pages.
    pub fn set_buttons_start(this: &mut WidgetMut<'_, Self>, buttons_start: u8) {
        this.widget.buttons_start = buttons_start;

        this.widget.sync_buttons(&mut this.ctx);
    }

    /// Sets the maximum number of buttons to always show for the last pages.
    pub fn set_buttons_end(this: &mut WidgetMut<'_, Self>, buttons_end: u8) {
        this.widget.buttons_end = buttons_end;

        this.widget.sync_buttons(&mut this.ctx);
    }

    /// Sets the maximum number of buttons shown in total.
    ///
    /// The effective button limit also depends on the total page count.
    pub fn set_buttons_total(this: &mut WidgetMut<'_, Self>, buttons_total: u8) {
        this.widget.buttons_total = buttons_total;

        this.widget.sync_buttons(&mut this.ctx);
    }
}

// --- MARK: METHODS
impl Pagination {
    /// Returns the list of page indices that the pagination buttons are associated with.
    fn derive_button_pages(&self) -> Vec<usize> {
        let button_count = self.page_count.min(self.buttons_total as usize);
        let mut button_pages = Vec::new();

        // Returns true if no more buttons can be added.
        let mut add_page = |idx| {
            if button_pages.len() < button_count
                && idx < self.page_count
                && !button_pages.contains(&idx)
            {
                button_pages.push(idx);
            }
            button_pages.len() == button_count
        };

        // Prioritize the currently active page.
        add_page(self.active_page);
        // Then all the first pages.
        for idx in 0..(self.buttons_start as usize) {
            add_page(idx);
        }
        // Followed by all the last pages.
        for idx in
            (self.page_count.saturating_sub(self.buttons_end as usize)..self.page_count).rev()
        {
            add_page(idx);
        }
        // Finally, reach the desired button count by alternating between next and previous pages.
        for idx in (1..).flat_map(|i| {
            [
                self.active_page.saturating_add(i),
                self.active_page.saturating_sub(i),
            ]
        }) {
            if add_page(idx) {
                break;
            }
        }

        // Make sure the chosen page indices are in ascending order.
        button_pages.sort();

        button_pages
    }

    /// Synchronizes all the child button widgets with the provided `button_pages`.
    fn sync_buttons<Ctx: SyncCtx>(&mut self, ctx: &mut Ctx) {
        // Derive the currently active page indices for buttons.
        let button_pages = self.derive_button_pages();

        // Remove any excess buttons that were created before the total limit was reduced.
        while self.buttons.len() > self.buttons_total as usize {
            let button = self.buttons.pop().unwrap();
            ctx.remove_child(button.widget);
        }
        // Stash all buttons that are within the limit but not in use.
        for button in self
            .buttons
            .iter_mut()
            .skip(button_pages.len())
            .filter(|button| !button.stashed)
        {
            button.stashed = true;
            ctx.set_stashed(&mut button.widget, true);
        }
        // Make sure all the existing buttons have the correct info.
        for (button, &page_idx) in self.buttons.iter_mut().zip(button_pages.iter()) {
            if button.page_idx != page_idx {
                button.page_idx = page_idx;
                let new_text = format!("{}", page_idx + 1);
                ctx.mutate_child_later(&mut button.widget, move |mut button| {
                    let mut btn_child = Button::child_mut(&mut button);
                    let mut btn_label = btn_child.downcast::<Label>();
                    Label::set_text(&mut btn_label, new_text);
                });
            }
            let disabled = button.page_idx == self.active_page;
            if button.disabled != disabled {
                button.disabled = disabled;
                ctx.mutate_child_later(&mut button.widget, move |mut button| {
                    button.ctx.set_disabled(disabled);
                });
            }
            if button.stashed {
                button.stashed = false;
                ctx.set_stashed(&mut button.widget, false);
            }
        }
        // Create any missing buttons.
        let mut children_changed = false;
        for page_idx in button_pages.into_iter().skip(self.buttons.len()) {
            let text = format!("{}", page_idx + 1);
            let disabled = page_idx == self.active_page;
            let button = Button::with_text(text);
            let button = NewWidget::new(button).disabled(disabled).to_pod();
            self.buttons.push(PageButton {
                widget: button,
                disabled,
                stashed: false,
                page_idx,
            });
            children_changed = true;
        }
        if children_changed {
            ctx.children_changed();
        }
    }
}

/// Page change action with the new page index.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PageChanged(pub usize);

// --- MARK: IMPL WIDGET
impl Widget for Pagination {
    type Action = PageChanged;

    fn on_action(
        &mut self,
        ctx: &mut ActionCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        action: &ErasedAction,
        source: WidgetId,
    ) {
        if action.is::<ButtonPress>()
            && let Some(button) = self
                .buttons
                .iter()
                .find(|button| button.widget.id() == source)
        {
            self.active_page = button.page_idx;
            self.sync_buttons(ctx);
            ctx.submit_action::<Self::Action>(PageChanged(self.active_page));
            ctx.set_handled();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for button in &mut self.buttons {
            ctx.register_child(&mut button.widget);
        }
    }

    fn property_changed(&mut self, _ctx: &mut UpdateCtx<'_>, _property_type: TypeId) {}

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                self.sync_buttons(ctx);
            }
            _ => (),
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        let mut length = 0.;

        let context_size = LayoutSize::maybe(axis.cross(), cross_length);

        for button in self.buttons.iter_mut().filter(|button| !button.stashed) {
            let auto_length = len_req.reduce(length).into();
            let button_length = ctx.compute_length(
                &mut button.widget,
                auto_length,
                context_size,
                axis,
                cross_length,
            );
            length = match axis {
                Axis::Horizontal => length + button_length,
                Axis::Vertical => length.max(button_length),
            };
        }

        length
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let mut used_width = 0.;
        for button in self.buttons.iter_mut().filter(|button| !button.stashed) {
            let space = Size::new((size.width - used_width).max(0.), size.height);
            let button_size =
                ctx.compute_size(&mut button.widget, SizeDef::fit(space), size.into());

            ctx.run_layout(&mut button.widget, button_size);

            let button_origin = Point::new(used_width, 0.);
            ctx.place_child(&mut button.widget, button_origin);

            used_width += button_size.width;
        }
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::Navigation
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
        // TODO: How to build the node?
        //       There doesn't seem to be any set_current_page or set_page_count.
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = ChildrenIds::with_capacity(self.buttons.len());
        for button in &self.buttons {
            ids.push(button.widget.id());
        }
        ids
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Pagination", id = id.trace())
    }
}

// --- MARK: SYNCCTX

/// Collections of context methods required to sync buttons.
trait SyncCtx {
    fn children_changed(&mut self);
    fn set_stashed(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>, stashed: bool);
    fn remove_child(&mut self, child: WidgetPod<impl Widget + ?Sized>);
    fn mutate_child_later<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        child: &mut WidgetPod<W>,
        f: impl FnOnce(WidgetMut<'_, W>) + Send + 'static,
    );
}

impl SyncCtx for MutateCtx<'_> {
    fn children_changed(&mut self) {
        self.children_changed();
    }

    fn set_stashed(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>, stashed: bool) {
        self.set_stashed(child, stashed);
    }

    fn remove_child(&mut self, child: WidgetPod<impl Widget + ?Sized>) {
        self.remove_child(child);
    }

    fn mutate_child_later<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        child: &mut WidgetPod<W>,
        f: impl FnOnce(WidgetMut<'_, W>) + Send + 'static,
    ) {
        self.mutate_child_later(child, f);
    }
}

impl SyncCtx for ActionCtx<'_> {
    fn children_changed(&mut self) {
        self.children_changed();
    }

    fn set_stashed(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>, stashed: bool) {
        self.set_stashed(child, stashed);
    }

    fn remove_child(&mut self, child: WidgetPod<impl Widget + ?Sized>) {
        self.remove_child(child);
    }

    fn mutate_child_later<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        child: &mut WidgetPod<W>,
        f: impl FnOnce(WidgetMut<'_, W>) + Send + 'static,
    ) {
        self.mutate_child_later(child, f);
    }
}

impl SyncCtx for UpdateCtx<'_> {
    fn children_changed(&mut self) {
        self.children_changed();
    }

    fn set_stashed(&mut self, child: &mut WidgetPod<impl Widget + ?Sized>, stashed: bool) {
        self.set_stashed(child, stashed);
    }

    fn remove_child(&mut self, child: WidgetPod<impl Widget + ?Sized>) {
        self.remove_child(child);
    }

    fn mutate_child_later<W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        child: &mut WidgetPod<W>,
        f: impl FnOnce(WidgetMut<'_, W>) + Send + 'static,
    ) {
        self.mutate_child_later(child, f);
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_pages() {
        let pg = Pagination::new(0);
        assert_eq!(pg.derive_button_pages(), vec![]);

        let pg = Pagination::new(1);
        assert_eq!(pg.derive_button_pages(), vec![0]);

        let pg = Pagination::new(2);
        assert_eq!(pg.derive_button_pages(), vec![0, 1]);

        let pg = Pagination::new(10)
            .buttons_total(0)
            .buttons_start(1)
            .buttons_end(1)
            .active_page(5);
        assert_eq!(pg.derive_button_pages(), vec![]);

        let pg = Pagination::new(10)
            .buttons_total(1)
            .buttons_start(1)
            .buttons_end(1)
            .active_page(5);
        assert_eq!(pg.derive_button_pages(), vec![5]);

        let pg = Pagination::new(10)
            .buttons_total(2)
            .buttons_start(1)
            .buttons_end(1)
            .active_page(5);
        assert_eq!(pg.derive_button_pages(), vec![0, 5]);

        let pg = Pagination::new(10)
            .buttons_total(3)
            .buttons_start(1)
            .buttons_end(1)
            .active_page(5);
        assert_eq!(pg.derive_button_pages(), vec![0, 5, 9]);

        let pg = Pagination::new(10)
            .buttons_total(5)
            .buttons_start(1)
            .buttons_end(1)
            .active_page(5);
        assert_eq!(pg.derive_button_pages(), vec![0, 4, 5, 6, 9]);

        let pg = Pagination::new(10)
            .buttons_total(5)
            .buttons_start(2)
            .buttons_end(2)
            .active_page(5);
        assert_eq!(pg.derive_button_pages(), vec![0, 1, 5, 8, 9]);

        let pg = Pagination::new(10)
            .buttons_total(5)
            .buttons_start(0)
            .buttons_end(2)
            .active_page(3);
        assert_eq!(pg.derive_button_pages(), vec![2, 3, 4, 8, 9]);

        let pg = Pagination::new(10)
            .buttons_total(5)
            .buttons_start(1)
            .buttons_end(1)
            .active_page(9);
        assert_eq!(pg.derive_button_pages(), vec![0, 6, 7, 8, 9]);

        let pg = Pagination::new(10)
            .buttons_total(20)
            .buttons_start(1)
            .buttons_end(1)
            .active_page(9);
        assert_eq!(pg.derive_button_pages(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
}
