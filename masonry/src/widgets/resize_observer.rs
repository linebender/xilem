// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::mem;

use crate::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx,
    Widget, WidgetMut, WidgetPod,
};
use crate::kurbo::{Axis, Point, Size};
use crate::layout::LenReq;

/// A widget which sends a [`LayoutChanged`] whenever its size changes.
///
/// It reports the child's length as its own in [`measure`], syncing its size with the child's.
///
/// The size of this widget can be accessed using [`MutateCtx::size`](crate::core::MutateCtx::size).
///
/// Ensure that `ResizeObserver` has [`Dimensions`] set via props to [`Dimensions::MAX`].
/// Max preferred size of `ResizeObserver` means that the question of size
/// will get passed through to its child, and doesn't mean that it will
/// necessarily map to the max preferred size of the child.
///
/// This can be a useful primitive for making size-adaptive designs, such as
/// scaling up a game board in response to more space being available, or switching
/// to use fewer columns when there is not enough space to fit multiple columns.
/// This can be safely used to dynamically access the size of a window
/// or tab in a [`Split`](crate::widgets::Split).
///
/// # Caveats
///
/// To avoid infinite loops, it is recommended to not use the reported size in a way
/// which will edit the child widget's size.
/// For example, using this to write the width of a label in that label would be
/// unlikely to reach a steady-state.
/// Currently Masonry will not detect these loops automatically, so using this
/// incorrectly might cause your application to stop responding.
///
/// You might also get several of the resulting actions in a sequence.
///
/// [`measure`]: Widget::measure
/// [`Dimensions`]: crate::properties::Dimensions
/// [`Dimensions::MAX`]: crate::properties::Dimensions::MAX
// TODO: It would be nice to at least catch these loops.
// We could see how many times layout is executed without us being painted, and setting a threshold.
// The response if that gets too high (100?) could be debug_panicking, then stopping
// sending size updates until we paint again.
// (This class of problem is the reason that we might wanted signal processing to happen
// in the mutate pass, so that its handling of infinite loops also applies to loops
// involving the driver)
pub struct ResizeObserver {
    child: WidgetPod<dyn Widget>,
    last_size: Option<Size>,
}

// --- MARK: BUILDERS
impl ResizeObserver {
    /// Creates a new resize observer, which will send [`LayoutChanged`] whenever its size changes.
    ///
    /// It reports the child's length as its own in [`measure`], syncing its size with the child's.
    ///
    /// [`measure`]: Widget::measure
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            last_size: None,
        }
    }
}

// --- MARK: WIDGETMUT
impl ResizeObserver {
    /// Replaces the child widget with a new one.
    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<impl Widget + ?Sized>) {
        let old_child = mem::replace(&mut this.widget.child, child.erased().to_pod());
        this.ctx.remove_child(old_child);
    }

    /// Forces this layout observer to send a new action.
    ///
    /// It's hard to imagine reasonable use cases for this method, but it's provided for completeness.
    pub fn force_resend(this: &mut WidgetMut<'_, Self>) {
        this.widget.last_size = None;
        this.ctx.request_layout();
    }

    /// Returns mutable reference to the child widget.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

/// The [action](Widget::Action) sent when the size of a widget has changed.
///
/// Currently only used by [`ResizeObserver`].
/// Note that this event does not itself include the final size.
/// That should instead be accessed through [`MutateCtx::size`](crate::core::MutateCtx::size).
#[derive(Debug)]
pub struct LayoutChanged;

// --- MARK: IMPL WIDGET
impl Widget for ResizeObserver {
    type Action = LayoutChanged;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<f64>,
    ) -> f64 {
        ctx.redirect_measurement(&mut self.child, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);

        let baseline = ctx.child_baseline_offset(&self.child);
        ctx.set_baseline_offset(baseline);

        if self.last_size.is_none_or(|it| it != size) {
            self.last_size = Some(size);
            ctx.submit_action::<Self::Action>(LayoutChanged);
        }
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

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use dpi::PhysicalSize;
    use masonry_testing::TestHarness;

    use crate::core::{NewWidget, Widget, WidgetTag, WindowEvent};
    use crate::kurbo::Size;
    use crate::layout::AsUnit;
    use crate::properties::Dimensions;
    use crate::theme::default_property_set;
    use crate::widgets::{Flex, LayoutChanged, ResizeObserver, SizedBox};

    #[test]
    fn detects_inner_resizing() {
        let tag = WidgetTag::named("inner_box");
        let inner_box =
            NewWidget::new_with_tag(SizedBox::empty().width(100.px()).height(100.px()), tag);
        let observer = ResizeObserver::new(inner_box).with_props(Dimensions::MAX);
        let observer_id = observer.id();
        // We use a flex here as the inner `SizedBox` will take up the full space available in this case.
        // This doesn't run into the caveat because the size of the inner widget is *not* based on the
        // size of the flex.
        let flex = Flex::column().with_fixed(observer).with_auto_id();
        let mut harness = TestHarness::create(default_property_set(), flex);
        // There will be an initial layout.
        let (LayoutChanged, action_id) = harness.pop_action::<LayoutChanged>().unwrap();
        assert_eq!(action_id, observer_id);
        assert_eq!(
            harness.get_widget_with_id(observer_id).ctx().size(),
            Size {
                width: 100.,
                height: 100.,
            }
        );
        // There shouldn't be a second layout.
        assert!(harness.pop_action::<LayoutChanged>().is_none());

        harness.edit_widget(tag, |mut it| SizedBox::set_height(&mut it, 200.px()));

        let (LayoutChanged, action_id) = harness.pop_action::<LayoutChanged>().unwrap();
        assert_eq!(action_id, observer_id);
        assert_eq!(
            harness.get_widget_with_id(observer_id).ctx().size(),
            Size {
                width: 100.,
                height: 200.,
            }
        );
        // There shouldn't be a second layout.
        assert!(harness.pop_action::<LayoutChanged>().is_none());

        // Resize to the same size.
        harness.edit_widget(tag, |mut it| SizedBox::set_height(&mut it, 200.px()));

        // The size hasn't changed, so no event.
        assert!(harness.pop_action::<LayoutChanged>().is_none());
    }

    #[test]
    fn detects_window_resizing() {
        let inner_box = SizedBox::empty().with_props(Dimensions::STRETCH);
        let observer = ResizeObserver::new(inner_box).with_props(Dimensions::MAX);
        let observer_id = observer.id();
        let mut harness = TestHarness::create_with_size(
            default_property_set(),
            observer,
            Size {
                width: 200.,
                height: 200.,
            },
        );
        // There will be an initial layout.
        let (LayoutChanged, action_id) = harness.pop_action::<LayoutChanged>().unwrap();
        assert_eq!(action_id, observer_id);
        assert_eq!(
            harness.get_widget_with_id(observer_id).ctx().size(),
            Size {
                width: 200.,
                height: 200.,
            }
        );
        // There shouldn't be a second layout.
        assert!(harness.pop_action::<LayoutChanged>().is_none());

        harness.process_window_event(WindowEvent::Resize(PhysicalSize::new(100, 150)));

        let (LayoutChanged, action_id) = harness.pop_action::<LayoutChanged>().unwrap();
        assert_eq!(action_id, observer_id);
        assert_eq!(
            harness.get_widget_with_id(observer_id).ctx().size(),
            Size {
                width: 100.,
                height: 150.,
            }
        );
        // There shouldn't be a second layout.
        assert!(harness.pop_action::<LayoutChanged>().is_none());

        // Same size again.
        harness.process_window_event(WindowEvent::Resize(PhysicalSize::new(100, 150)));

        // The size hasn't changed, so no event.
        assert!(harness.pop_action::<LayoutChanged>().is_none());
    }
}
