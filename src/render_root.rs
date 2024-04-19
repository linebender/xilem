use std::collections::VecDeque;

// Automatically defaults to std::time::Instant on non Wasm platforms
use instant::Instant;
use kurbo::Affine;
use parley::FontContext;
use tracing::{info_span, warn};
use vello::peniko::{Color, Fill};
use vello::Scene;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::window::CursorIcon;

use crate::contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, WidgetCtx, WorkerFn};
use crate::debug_logger::DebugLogger;
use crate::event::{PointerEvent, TextEvent, WindowEvent};
use crate::kurbo::Point;
use crate::widget::{FocusChange, StoreInWidgetMut, WidgetMut, WidgetState};
use crate::{
    Action, BoxConstraints, Handled, InternalLifeCycle, LifeCycle, Widget, WidgetId, WidgetPod,
};

// TODO - Remove pub(crate)
pub struct RenderRoot {
    pub(crate) root: WidgetPod<Box<dyn Widget>>,
    pub(crate) size_policy: WindowSizePolicy,
    pub(crate) size: PhysicalSize<u32>,
    // TODO - Currently this is always 1.0
    // kurbo coordinates are assumed to be in logical pixels
    pub(crate) scale_factor: f64,
    /// Is `Some` if the most recently displayed frame was an animation frame.
    pub(crate) last_anim: Option<Instant>,
    pub(crate) last_mouse_pos: Option<PhysicalPosition<f64>>,
    pub(crate) cursor_icon: CursorIcon,
    pub(crate) state: RenderRootState,
}

pub(crate) struct RenderRootState {
    pub(crate) debug_logger: DebugLogger,
    pub(crate) signal_queue: VecDeque<RenderRootSignal>,
    pub(crate) focused_widget: Option<WidgetId>,
    pub(crate) font_context: FontContext,
}

/// Defines how a windows size should be determined
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum WindowSizePolicy {
    /// Use the content of the window to determine the size.
    ///
    /// If you use this option, your root widget will be passed infinite constraints;
    /// you are responsible for ensuring that your content picks an appropriate size.
    Content,
    /// Use the provided window size.
    #[default]
    User,
}

// TODO - Handle custom cursors?
// TODO - handling timers
// TODO - Text fields
pub enum RenderRootSignal {
    Action(Action, WidgetId),
    TextFieldAdded,
    TextFieldRemoved,
    TextFieldFocused,
    ImeStarted,
    ImeMoved,
    ImeInvalidated,
    RequestRedraw,
    RequestAnimFrame,
    SpawnWorker(WorkerFn),
    TakeFocus,
    SetCursor(CursorIcon),
    SetSize(PhysicalSize<u32>),
    SetTitle(String),
}

impl RenderRoot {
    pub fn new(root_widget: impl Widget, size_policy: WindowSizePolicy) -> Self {
        let mut root = RenderRoot {
            root: WidgetPod::new(root_widget).boxed(),
            size_policy,
            size: PhysicalSize::new(0, 0),
            scale_factor: 1.0,
            last_anim: None,
            last_mouse_pos: None,
            cursor_icon: CursorIcon::Default,
            state: RenderRootState {
                debug_logger: DebugLogger::new(false),
                signal_queue: VecDeque::new(),
                focused_widget: None,
                font_context: FontContext::default(),
            },
        };

        // We send WidgetAdded to all widgets right away
        root.root_lifecycle(LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded));

        // We run a layout pass right away to have a SetSize signal ready
        if size_policy == WindowSizePolicy::Content {
            root.root_layout();
        }

        root
    }

    pub fn handle_window_event(&mut self, event: WindowEvent) -> Handled {
        match event {
            WindowEvent::Rescale(_scale) => {
                // TODO - Handle dpi scaling
                // For now we're assuming the scale factor is always 1.0
                Handled::No
            }
            WindowEvent::Resize(size) => {
                self.size = size;
                self.root.state.needs_layout = true;
                self.state
                    .signal_queue
                    .push_back(RenderRootSignal::RequestRedraw);
                Handled::No
            }
            WindowEvent::AnimFrame => {
                let now = Instant::now();
                // TODO: this calculation uses wall-clock time of the paint call, which
                // potentially has jitter.
                //
                // See https://github.com/linebender/druid/issues/85 for discussion.
                let last = self.last_anim.take();
                let elapsed_ns = last.map(|t| now.duration_since(t).as_nanos()).unwrap_or(0) as u64;

                if self.wants_animation_frame() {
                    self.root_lifecycle(LifeCycle::AnimFrame(elapsed_ns));
                    self.last_anim = Some(now);
                }
                Handled::Yes
            }
        }
    }

    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> Handled {
        self.root_on_pointer_event(event)
    }

    pub fn handle_text_event(&mut self, event: TextEvent) -> Handled {
        self.root_on_text_event(event)
    }

    pub fn redraw(&mut self) -> Scene {
        // TODO - Xilem's reconciliation logic will have to be called
        // by the function that calls this

        // TODO - if root widget's request_anim is still set by the
        // time this is called, emit a warning
        if self.root.state().needs_layout {
            self.root_layout();
        }
        if self.root.state().needs_layout {
            warn!("Widget requested layout during layout pass");
            self.state
                .signal_queue
                .push_back(RenderRootSignal::RequestRedraw);
        }

        // TODO - Improve caching of scenes.
        self.root_paint()
    }

    pub fn pop_signal(&mut self) -> Option<RenderRootSignal> {
        self.state.signal_queue.pop_front()
    }

    pub fn pop_signal_matching(
        &mut self,
        predicate: impl Fn(&RenderRootSignal) -> bool,
    ) -> Option<RenderRootSignal> {
        let idx = self.state.signal_queue.iter().position(predicate)?;
        self.state.signal_queue.remove(idx)
    }

    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    pub fn edit_root_widget<R>(
        &mut self,
        f: impl FnOnce(WidgetMut<'_, Box<dyn Widget>>) -> R,
    ) -> R {
        let mut fake_widget_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let root_widget = WidgetMut {
            inner: Box::<dyn Widget>::from_widget_and_ctx(
                &mut self.root.inner,
                WidgetCtx {
                    global_state: &mut self.state,
                    widget_state: &mut self.root.state,
                },
            ),
            parent_widget_state: &mut fake_widget_state,
        };

        let res = f(root_widget);
        self.post_event_processing(&mut fake_widget_state);

        res
    }

    fn root_on_pointer_event(&mut self, event: PointerEvent) -> Handled {
        let mut widget_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");

        let mut ctx = EventCtx {
            global_state: &mut self.state,
            widget_state: &mut widget_state,
            is_handled: false,
            request_pan_to_child: None,
        };

        // TODO - Only for primary pointer
        self.last_mouse_pos = match event {
            PointerEvent::PointerLeave(_) | PointerEvent::HoverFile(_, _) => None,
            _ => Some(event.pointer_state().position),
        };

        let handled = {
            ctx.global_state
                .debug_logger
                .push_important_span(&format!("¨POINTER_EVENT {}", event.short_name()));
            let _span = info_span!("event").entered();
            self.root.on_pointer_event(&mut ctx, &event);
            ctx.global_state.debug_logger.pop_span();
            Handled::from(ctx.is_handled)
        };

        if let Some(cursor) = &ctx.widget_state.cursor {
            // TODO - Add methods and `into()` impl to make this more concise.
            ctx.global_state
                .signal_queue
                .push_back(RenderRootSignal::SetCursor(*cursor));
        } else {
            ctx.global_state
                .signal_queue
                .push_back(RenderRootSignal::SetCursor(CursorIcon::Default));
        }

        self.post_event_processing(&mut widget_state);
        self.root.as_dyn().debug_validate(false);

        handled
    }

    fn root_on_text_event(&mut self, event: TextEvent) -> Handled {
        let mut widget_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");

        let mut ctx = EventCtx {
            global_state: &mut self.state,
            widget_state: &mut widget_state,
            is_handled: false,
            request_pan_to_child: None,
        };

        let handled = {
            ctx.global_state
                .debug_logger
                .push_important_span(&format!("TEXT_EVENT {}", event.short_name()));
            let _span = info_span!("event").entered();
            self.root.on_text_event(&mut ctx, &event);
            ctx.global_state.debug_logger.pop_span();
            Handled::from(ctx.is_handled)
        };

        self.post_event_processing(&mut widget_state);
        self.root.as_dyn().debug_validate(false);

        handled
    }

    fn root_lifecycle(&mut self, event: LifeCycle) {
        let mut widget_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let mut ctx = LifeCycleCtx {
            global_state: &mut self.state,
            widget_state: &mut widget_state,
        };

        {
            ctx.global_state
                .debug_logger
                .push_important_span(&format!("LIFECYCLE {}", event.short_name()));
            let _span = info_span!("lifecycle").entered();
            self.root.lifecycle(&mut ctx, &event);
            self.state.debug_logger.pop_span();
        }

        // TODO - Remove this line
        // post_event_processing can recursively call root_lifecycle, which
        // makes the execution model more complex and unpredictable.
        self.post_event_processing(&mut widget_state);
    }

    pub(crate) fn root_layout(&mut self) {
        let mut widget_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let size = self.get_kurbo_size();
        let mouse_pos = self.last_mouse_pos.map(|pos| (pos.x, pos.y).into());
        let mut layout_ctx = LayoutCtx {
            global_state: &mut self.state,
            widget_state: &mut widget_state,
            mouse_pos,
        };

        let bc = match self.size_policy {
            WindowSizePolicy::User => BoxConstraints::tight(size),
            WindowSizePolicy::Content => BoxConstraints::UNBOUNDED,
        };

        let size = {
            layout_ctx
                .global_state
                .debug_logger
                .push_important_span("LAYOUT");
            let _span = info_span!("layout").entered();
            self.root.layout(&mut layout_ctx, &bc)
        };
        layout_ctx.global_state.debug_logger.pop_span();

        if let WindowSizePolicy::Content = self.size_policy {
            let new_size = LogicalSize::new(size.width, size.height).to_physical(self.scale_factor);
            if self.size != new_size {
                self.size = new_size;
                layout_ctx
                    .global_state
                    .signal_queue
                    .push_back(RenderRootSignal::SetSize(new_size));
            }
        }

        layout_ctx.place_child(&mut self.root, Point::ORIGIN);
        self.root_lifecycle(LifeCycle::Internal(InternalLifeCycle::ParentWindowOrigin));
        self.post_event_processing(&mut widget_state);
    }

    fn root_paint(&mut self) -> Scene {
        // TODO - Handle Xilem's VIEW_CONTEXT_CHANGED

        let widget_state = WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let mut ctx = PaintCtx {
            global_state: &mut self.state,
            widget_state: &widget_state,
            depth: 0,
            debug_paint: false,
            debug_widget: false,
        };

        let mut scene = Scene::new();
        self.root.paint(&mut ctx, &mut scene);

        // FIXME - This is a workaround to Vello panicking when given an
        // empty scene
        // See https://github.com/linebender/vello/issues/291
        let empty_path = kurbo::Rect::ZERO;
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::TRANSPARENT,
            None,
            &empty_path,
        );

        scene
    }

    fn get_kurbo_size(&self) -> kurbo::Size {
        let size = self.size.to_logical(self.scale_factor);
        kurbo::Size::new(size.width, size.height)
    }

    fn post_event_processing(&mut self, widget_state: &mut WidgetState) {
        // If children are changed during the handling of an event,
        // we need to send RouteWidgetAdded now, so that they are ready for update/layout.
        if widget_state.children_changed {
            // TODO - Update IME handlers
            // Send TextFieldRemoved signal

            self.root_lifecycle(LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded));
        }

        if self.state.debug_logger.layout_tree.root.is_none() {
            self.state.debug_logger.layout_tree.root = Some(self.root.id().to_raw() as u32);
        }

        if self.root.state().needs_window_origin && !self.root.state().needs_layout {
            let event = LifeCycle::Internal(InternalLifeCycle::ParentWindowOrigin);
            self.root_lifecycle(event);
        }

        // Update the disabled state if necessary
        // Always do this before updating the focus-chain
        if self.root.state().tree_disabled_changed() {
            let event = LifeCycle::Internal(InternalLifeCycle::RouteDisabledChanged);
            self.root_lifecycle(event);
        }

        // Update the focus-chain if necessary
        // Always do this before sending focus change, since this event updates the focus chain.
        if self.root.state().update_focus_chain {
            let event = LifeCycle::BuildFocusChain;
            self.root_lifecycle(event);
        }

        self.update_focus(widget_state);

        // If we need a new paint pass, make sure winit knows it.
        if self.wants_animation_frame() {
            self.state
                .signal_queue
                .push_back(RenderRootSignal::RequestAnimFrame);
        }

        if self.root.state().needs_paint {
            self.state
                .signal_queue
                .push_back(RenderRootSignal::RequestRedraw);
        }

        #[cfg(FALSE)]
        for ime_field in widget_state.text_registrations.drain(..) {
            let token = self.handle.add_text_field();
            tracing::debug!("{:?} added", token);
            self.ime_handlers.push((token, ime_field));
        }
    }

    /// `true` iff any child requested an animation frame since the last `AnimFrame` event.
    fn wants_animation_frame(&self) -> bool {
        self.root.state().request_anim
    }

    fn update_focus(&mut self, widget_state: &mut WidgetState) {
        if let Some(focus_req) = widget_state.request_focus.take() {
            let old = self.state.focused_widget;
            let new = self.widget_for_focus_request(focus_req);

            // TODO
            // Skip change if requested widget is disabled

            // Only send RouteFocusChanged in case there's actual change
            if old != new {
                let event = LifeCycle::Internal(InternalLifeCycle::RouteFocusChanged { old, new });
                self.root_lifecycle(event);
                self.state.focused_widget = new;

                // TODO - Handle IME
                // Send TextFieldFocused(focused_widget) signal
            }
        }
    }

    fn widget_for_focus_request(&self, focus: FocusChange) -> Option<WidgetId> {
        match focus {
            FocusChange::Resign => None,
            FocusChange::Focus(id) => Some(id),
            FocusChange::Next => self.widget_from_focus_chain(true),
            FocusChange::Previous => self.widget_from_focus_chain(false),
        }
    }

    fn widget_from_focus_chain(&self, forward: bool) -> Option<WidgetId> {
        self.state.focused_widget.and_then(|focus| {
            self.focus_chain()
                .iter()
                // Find where the focused widget is in the focus chain
                .position(|id| id == &focus)
                .map(|idx| {
                    // Return the id that's next to it in the focus chain
                    let len = self.focus_chain().len();
                    let new_idx = if forward {
                        (idx + 1) % len
                    } else {
                        (idx + len - 1) % len
                    };
                    self.focus_chain()[new_idx]
                })
                .or_else(|| {
                    // If the currently focused widget isn't in the focus chain,
                    // then we'll just return the first/last entry of the chain, if any.
                    if forward {
                        self.focus_chain().first().copied()
                    } else {
                        self.focus_chain().last().copied()
                    }
                })
        })
    }

    // TODO - Store in RenderRootState
    pub(crate) fn focus_chain(&self) -> &[WidgetId] {
        &self.root.state().focus_chain
    }
}

/*
TODO:
- Invalidation regions
- Timer handling
- prepare_paint
- Focus-related stuff
*/
