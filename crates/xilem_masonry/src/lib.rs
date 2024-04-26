#![allow(clippy::comparison_chain)]
use std::{any::Any, collections::HashMap};

use masonry::{
    app_driver::AppDriver,
    event_loop_runner::EventLoopRunner,
    widget::{StoreInWidgetMut, WidgetMut, WidgetRef},
    BoxConstraints, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, PointerEvent,
    Size, StatusChange, TextEvent, Widget, WidgetId, WidgetPod,
};
use smallvec::SmallVec;
use vello::Scene;
use winit::{
    dpi::LogicalSize,
    error::EventLoopError,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod any_view;
mod id;
mod sequence;
mod vec_splice;
pub use any_view::{AnyMasonryView, BoxedMasonryView};
pub mod view;
pub use id::ViewId;
pub use sequence::{ElementSplice, ViewSequence};
pub use vec_splice::VecSplice;

pub struct Xilem<AppState, Logic, View>
where
    View: MasonryView<AppState>,
{
    root_widget: RootWidget<View::Element>,
    driver: MasonryDriver<AppState, Logic, View>,
}

pub struct MasonryDriver<AppState, Logic, View: MasonryView<AppState, ()>> {
    app_state: AppState,
    view_state: View::State,
    root_view_id: ViewId,
    logic: Logic,
    root_view: View,
    view_cx: ViewCx,
}

// TODO: This is a hack to work around pod-racing
// TODO: `declare_widget` *forces* this to be pub
pub struct RootWidget<E> {
    pub(crate) pod: WidgetPod<E>,
}

masonry::declare_widget!(RootWidgetMut, RootWidget<E: (Widget)>);

impl<E: 'static + Widget> Widget for RootWidget<E> {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        self.pod.on_pointer_event(ctx, event);
    }
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.pod.on_text_event(ctx, event);
    }

    fn on_status_change(&mut self, _: &mut LifeCycleCtx, _: &StatusChange) {
        // Intentionally do nothing?
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.pod.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = self.pod.layout(ctx, bc);
        ctx.place_child(&mut self.pod, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.pod.paint(ctx, scene);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        let mut vec = SmallVec::new();
        vec.push(self.pod.as_dyn());
        vec
    }
}

impl<AppState, Logic, View> AppDriver for MasonryDriver<AppState, Logic, View>
where
    Logic: FnMut(&mut AppState) -> View,
    View: MasonryView<AppState>,
{
    fn on_action(
        &mut self,
        ctx: &mut masonry::app_driver::DriverCtx<'_>,
        widget_id: masonry::WidgetId,
        action: masonry::Action,
    ) {
        if let Some(id_path) = self.view_cx.widget_map.get(&widget_id) {
            let message_result = self.root_view.message(
                id_path.as_slice(),
                &mut self.view_state,
                Box::new(action),
                &mut self.app_state,
            );
            let rebuild = match message_result {
                MessageResult::Action(()) => {
                    // It's not entirely clear what to do here
                    true
                }
                MessageResult::RequestRebuild => true,
                MessageResult::Nop => false,
                MessageResult::Stale(_) => {
                    tracing::info!("Discarding message");
                    false
                }
            };
            if rebuild {
                let next_view = (self.logic)(&mut self.app_state);
                let mut root = ctx.get_root::<RootWidget<View::Element>>();
                let element = root.get_element();

                let changed = next_view.rebuild(
                    &mut self.view_cx,
                    &self.root_view,
                    &mut self.root_view_id,
                    &mut self.view_state,
                    element,
                );
                if !changed.changed {
                    // Masonry manages all of this itself - ChangeFlags is probably not needed?
                    tracing::debug!("TODO: Skip some work?");
                }
                self.root_view = next_view;
            }
        } else {
            tracing::error!("Got action {action:?} for unknown widget. Did you forget to use `with_action_widget`?");
        }
    }
}

impl<E: Widget + StoreInWidgetMut> RootWidgetMut<'_, E> {
    pub fn get_element(&mut self) -> WidgetMut<'_, E> {
        self.ctx.get_mut(&mut self.widget.pod)
    }
}

impl<AppState, Logic, View> Xilem<AppState, Logic, View>
where
    Logic: FnMut(&mut AppState) -> View,
    View: MasonryView<AppState>,
{
    pub fn new(mut app_state: AppState, mut logic: Logic) -> Self {
        let first_view = logic(&mut app_state);
        let mut view_cx = ViewCx {
            id_path: vec![],
            widget_map: HashMap::new(),
        };
        let (root_view_id, view_state, pod) = first_view.build(&mut view_cx);
        let root_widget = RootWidget { pod };
        Xilem {
            driver: MasonryDriver {
                root_view: first_view,
                root_view_id,
                view_state,
                logic,
                app_state,
                view_cx,
            },
            root_widget,
        }
    }

    // TODO: Make windows a specific view
    pub fn run_windowed(self, window_title: String) -> Result<(), EventLoopError>
    where
        AppState: 'static,
        Logic: 'static,
        View: 'static,
    {
        let event_loop = EventLoop::new().unwrap();
        let window_size = LogicalSize::new(600., 800.);
        let window = WindowBuilder::new()
            .with_title(window_title)
            .with_resizable(true)
            .with_min_inner_size(window_size)
            .build(&event_loop)
            .unwrap();
        self.run_windowed_in(window, event_loop)
    }

    // TODO: Make windows into a custom view
    pub fn run_windowed_in(
        self,
        window: Window,
        event_loop: EventLoop<()>,
    ) -> Result<(), EventLoopError>
    where
        AppState: 'static,
        Logic: 'static,
        View: 'static,
    {
        EventLoopRunner::new(self.root_widget, window, event_loop, self.driver).run()
    }
}
pub trait MasonryView<AppState, Action = ()>: Send + 'static {
    /// Associated state for the view.
    type State;
    /// The associated element for the view.
    type Element: Widget + StoreInWidgetMut;
    fn build(&self, cx: &mut ViewCx) -> (ViewId, Self::State, WidgetPod<Self::Element>);

    fn rebuild(
        &self,
        _cx: &mut ViewCx,
        prev: &Self,
        id: &mut ViewId,
        state: &mut Self::State,
        element: WidgetMut<Self::Element>,
    ) -> ChangeFlags;

    fn message(
        &self,
        id_path: &[ViewId],
        state: &mut Self::State,
        message: Box<dyn Any>,
        app_state: &mut AppState,
    ) -> MessageResult<Action>;
}

#[must_use]
pub struct ChangeFlags {
    changed: bool,
}

impl ChangeFlags {
    const CHANGED: Self = ChangeFlags { changed: true };
    const UNCHANGED: Self = ChangeFlags { changed: false };
}

pub struct ViewCx {
    /// The map from a widgets id to its position in the View tree.
    ///
    /// This includes only the widgets which might send actions
    /// This is currently never cleaned up
    widget_map: HashMap<WidgetId, Vec<ViewId>>,
    id_path: Vec<ViewId>,
}

impl ViewCx {
    pub fn with_action_widget<V:'static, E: Widget>(
        &mut self,
        f: impl FnOnce(&mut Self) -> WidgetPod<E>,
    ) -> (ViewId, WidgetPod<E>) {
        self.with_new_id::<V, _, _>(|cx| {
            let value = f(cx);
            let id = value.id();
            let path = cx.id_path.clone();
            cx.widget_map.insert(id, path);
            value
        })
    }

    /// Run some logic with an id added to the id path.
    ///
    /// This is an ergonomic helper that ensures proper nesting of the id path.
    pub fn with_id<R>(&mut self, id: ViewId, f: impl FnOnce(&mut Self) -> R) -> R {
        self.id_path.push(id);
        let res = f(self);
        self.id_path.pop();
        res
    }

    /// Allocate a new id and run logic with the new id added to the id path.
    ///
    /// Also an ergonomic helper.
    pub fn with_new_id<V: 'static, T, F: FnOnce(&mut ViewCx) -> T>(&mut self, f: F) -> (ViewId, T) {
        // Note: Currently this requires an extra generic param `V` to be `'static`,
        // which is not really necessary (only for debugging),
        // so in case this causes issues, we can just not use that debugging information (i.e. `ViewId::next()`)
        let id = ViewId::next_with_type::<V>();
        self.id_path.push(id);
        let result = f(self);
        self.id_path.pop();
        (id, result)
    }
}

/// A result wrapper type for event handlers.
#[derive(Default)]
pub enum MessageResult<A> {
    Action(A),
    RequestRebuild,
    #[default]
    Nop,
    Stale(Box<dyn Any>),
}

impl<A> MessageResult<A> {
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> MessageResult<B> {
        match self {
            MessageResult::Action(a) => MessageResult::Action(f(a)),
            MessageResult::RequestRebuild => MessageResult::RequestRebuild,
            MessageResult::Stale(event) => MessageResult::Stale(event),
            MessageResult::Nop => MessageResult::Nop,
        }
    }

    pub fn or(self, f: impl FnOnce(Box<dyn Any>) -> Self) -> Self {
        match self {
            MessageResult::Stale(event) => f(event),
            _ => self,
        }
    }
}
