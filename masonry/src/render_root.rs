// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::VecDeque;

use accesskit::{ActionRequest, NodeBuilder, Tree, TreeUpdate};
use kurbo::Affine;
use parley::{FontContext, LayoutContext};
use tracing::{debug, info_span, warn};
use vello::peniko::{Color, Fill};
use vello::Scene;
use winit::keyboard::{KeyCode, PhysicalKey};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, WidgetCtx, WorkerFn};
use crate::debug_logger::DebugLogger;
use crate::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use crate::event::{PointerEvent, TextEvent, WindowEvent};
use crate::kurbo::Point;
use crate::text2::TextBrush;
use crate::tree_arena::TreeArena;
use crate::widget::{WidgetMut, WidgetRef, WidgetState};
use crate::{
    AccessCtx, AccessEvent, Action, BoxConstraints, CursorIcon, Handled, InternalLifeCycle,
    LifeCycle, Widget, WidgetId, WidgetPod,
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
    pub(crate) last_mouse_pos: Option<LogicalPosition<f64>>,
    pub(crate) cursor_icon: CursorIcon,
    pub(crate) state: RenderRootState,
    // TODO - Add "access_tree_active" to detect when you don't need to update the
    // access tree
    pub(crate) rebuild_access_tree: bool,
    pub(crate) widget_arena: WidgetArena,
}

pub(crate) struct RenderRootState {
    pub(crate) debug_logger: DebugLogger,
    pub(crate) signal_queue: VecDeque<RenderRootSignal>,
    pub(crate) focused_widget: Option<WidgetId>,
    pub(crate) next_focused_widget: Option<WidgetId>,
    pub(crate) font_context: FontContext,
    pub(crate) text_layout_context: LayoutContext<TextBrush>,
}

pub(crate) struct WidgetArena {
    pub(crate) widgets: TreeArena<Box<dyn Widget>>,
    pub(crate) widget_states: TreeArena<WidgetState>,
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
    StartIme,
    EndIme,
    ImeMoved(LogicalPosition<f64>, LogicalSize<f64>),
    RequestRedraw,
    RequestAnimFrame,
    SpawnWorker(WorkerFn),
    TakeFocus,
    SetCursor(CursorIcon),
    SetSize(PhysicalSize<u32>),
    SetTitle(String),
}

impl RenderRoot {
    pub fn new(root_widget: impl Widget, size_policy: WindowSizePolicy, scale_factor: f64) -> Self {
        let mut root = RenderRoot {
            root: WidgetPod::new(root_widget).boxed(),
            size_policy,
            size: PhysicalSize::new(0, 0),
            scale_factor,
            last_anim: None,
            last_mouse_pos: None,
            cursor_icon: CursorIcon::Default,
            state: RenderRootState {
                debug_logger: DebugLogger::new(false),
                signal_queue: VecDeque::new(),
                focused_widget: None,
                next_focused_widget: None,
                font_context: FontContext::default(),
                text_layout_context: LayoutContext::new(),
            },
            widget_arena: WidgetArena {
                widgets: TreeArena::new(),
                widget_states: TreeArena::new(),
            },
            rebuild_access_tree: true,
        };

        // We send WidgetAdded to all widgets right away
        root.root_lifecycle(LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded));

        // We run a layout pass right away to have a SetSize signal ready
        if size_policy == WindowSizePolicy::Content {
            root.root_layout();
        }

        root
    }

    fn root_state(&mut self) -> &mut WidgetState {
        self.widget_arena
            .widget_states
            .root_token_mut()
            .into_child_mut(self.root.id().to_raw())
            .expect("root widget not in widget tree")
            .0
    }

    // --- MARK: WINDOW_EVENT ---
    pub fn handle_window_event(&mut self, event: WindowEvent) -> Handled {
        match event {
            WindowEvent::Rescale(scale_factor) => {
                self.scale_factor = scale_factor;
                // TODO - What we'd really like is to request a repaint and an accessibility
                // pass for every single widget.
                self.root_state().needs_layout = true;
                self.state
                    .signal_queue
                    .push_back(RenderRootSignal::RequestRedraw);
                Handled::Yes
            }
            WindowEvent::Resize(size) => {
                self.size = size;
                self.root_state().needs_layout = true;
                self.state
                    .signal_queue
                    .push_back(RenderRootSignal::RequestRedraw);
                Handled::Yes
            }
            WindowEvent::AnimFrame => {
                let now = Instant::now();
                // TODO: this calculation uses wall-clock time of the paint call, which
                // potentially has jitter.
                //
                // See https://github.com/linebender/druid/issues/85 for discussion.
                let last = self.last_anim.take();
                let elapsed_ns = last.map(|t| now.duration_since(t).as_nanos()).unwrap_or(0) as u64;

                if self.root_state().request_anim {
                    self.root_lifecycle(LifeCycle::AnimFrame(elapsed_ns));
                    self.last_anim = Some(now);
                }
                Handled::Yes
            }
            WindowEvent::RebuildAccessTree => {
                self.rebuild_access_tree = true;
                self.state
                    .signal_queue
                    .push_back(RenderRootSignal::RequestRedraw);
                Handled::Yes
            }
        }
    }

    // --- MARK: PUB FUNCTIONS ---
    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> Handled {
        self.root_on_pointer_event(event)
    }

    pub fn handle_text_event(&mut self, event: TextEvent) -> Handled {
        self.root_on_text_event(event)
    }

    pub fn redraw(&mut self) -> (Scene, TreeUpdate) {
        // TODO - Xilem's reconciliation logic will have to be called
        // by the function that calls this

        // TODO - if root widget's request_anim is still set by the
        // time this is called, emit a warning
        if self.root_state().needs_layout {
            self.root_layout();
        }
        if self.root_state().needs_layout {
            warn!("Widget requested layout during layout pass");
            self.state
                .signal_queue
                .push_back(RenderRootSignal::RequestRedraw);
        }

        // TODO - Improve caching of scenes.
        (self.root_paint(), self.root_accessibility())
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

    // --- MARK: GET ROOT---
    pub fn get_root_widget(&self) -> WidgetRef<dyn Widget> {
        let root_state_token = self.widget_arena.widget_states.root_token();
        let root_widget_token = self.widget_arena.widgets.root_token();
        let (state, state_children) = root_state_token
            .into_child(self.root.id().to_raw())
            .expect("root widget not in widget tree");
        let (widget, widget_children) = root_widget_token
            .into_child(self.root.id().to_raw())
            .expect("root widget not in widget tree");

        // Our WidgetArena stores all widgets as Box<dyn Widget>, but the "true"
        // type of our root widget is *also* Box<dyn Widget>. We downcast so we
        // don't add one more level of indirection to this.
        let widget = widget
            .as_dyn_any()
            .downcast_ref::<Box<dyn Widget>>()
            .unwrap();

        WidgetRef {
            widget_state_children: state_children,
            widget_children,
            widget_state: state,
            widget,
        }
    }

    pub fn edit_root_widget<R>(
        &mut self,
        f: impl FnOnce(WidgetMut<'_, Box<dyn Widget>>) -> R,
    ) -> R {
        let mut fake_widget_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();
        let (state, state_token) = root_state_token
            .into_child_mut(self.root.id().to_raw())
            .expect("root widget not in widget tree");
        let (widget, widget_token) = root_widget_token
            .into_child_mut(self.root.id().to_raw())
            .expect("root widget not in widget tree");

        // Our WidgetArena stores all widgets as Box<dyn Widget>, but the "true"
        // type of our root widget is *also* Box<dyn Widget>. We downcast so we
        // don't add one more level of indirection to this.
        let widget = widget
            .as_mut_dyn_any()
            .downcast_mut::<Box<dyn Widget>>()
            .unwrap();

        self.state.next_focused_widget = self.state.focused_widget;
        let root_widget = WidgetMut {
            ctx: WidgetCtx {
                global_state: &mut self.state,
                parent_widget_state: &mut fake_widget_state,
                widget_state: state,
                widget_state_children: state_token,
                widget_children: widget_token,
            },
            widget,
        };

        let res = {
            let _span = info_span!("edit_root_widget").entered();
            f(root_widget)
        };
        self.post_event_processing(&mut fake_widget_state);

        res
    }

    pub fn get_widget(&self, id: WidgetId) -> Option<WidgetRef<dyn Widget>> {
        let (state, state_token) = self.widget_arena.widget_states.find(id.to_raw())?;
        let (widget, widget_token) = self
            .widget_arena
            .widgets
            .find(id.to_raw())
            .expect("found state but not widget");

        // Box<dyn Widget> -> &dyn Widget
        // Without this step, the type of `WidgetRef::widget` would be
        // `&Box<dyn Widget> as &dyn Widget`, which would be an additional layer
        // of indirection.
        let widget: &dyn Widget = &**widget;
        Some(WidgetRef {
            widget_state_children: state_token,
            widget_children: widget_token,
            widget_state: state,
            widget,
        })
    }

    // --- MARK: POINTER_EVENT ---
    fn root_on_pointer_event(&mut self, event: PointerEvent) -> Handled {
        let mut root_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");

        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();

        self.state.next_focused_widget = self.state.focused_widget;
        let mut ctx = EventCtx {
            global_state: &mut self.state,
            widget_state: &mut root_state,
            widget_state_children: root_state_token,
            widget_children: root_widget_token,
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
                .push_important_span(&format!("POINTER_EVENT {}", event.short_name()));
            let _span = info_span!("pointer_event").entered();
            if !event.is_high_density() {
                debug!("Running ON_POINTER_EVENT pass with {}", event.short_name());
            }

            self.root.on_pointer_event(&mut ctx, &event);

            if !event.is_high_density() {
                debug!(
                    focused_widget = ctx.global_state.focused_widget.map(|id| id.0),
                    handled = ctx.is_handled,
                    "ON_POINTER_EVENT finished",
                );
            }
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

        self.post_event_processing(&mut root_state);
        self.get_root_widget().debug_validate(false);

        handled
    }

    // --- MARK: TEXT_EVENT ---
    fn root_on_text_event(&mut self, event: TextEvent) -> Handled {
        let mut root_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");

        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();

        self.state.next_focused_widget = self.state.focused_widget;
        let mut ctx = EventCtx {
            global_state: &mut self.state,
            widget_state: &mut root_state,
            widget_state_children: root_state_token,
            widget_children: root_widget_token,
            is_handled: false,
            request_pan_to_child: None,
        };

        let handled = {
            ctx.global_state
                .debug_logger
                .push_important_span(&format!("TEXT_EVENT {}", event.short_name()));
            let _span = info_span!("text_event").entered();
            if !event.is_high_density() {
                debug!("Running ON_TEXT_EVENT pass with {}", event.short_name());
            }

            self.root.on_text_event(&mut ctx, &event);

            if !event.is_high_density() {
                debug!(
                    focused_widget = ctx.global_state.focused_widget.map(|id| id.0),
                    handled = ctx.is_handled,
                    "ON_TEXT_EVENT finished",
                );
            }
            ctx.global_state.debug_logger.pop_span();
            Handled::from(ctx.is_handled)
        };

        // If event is tab we handle focus
        if let TextEvent::KeyboardKey(key, mods) = event {
            if handled == Handled::No && key.physical_key == PhysicalKey::Code(KeyCode::Tab) {
                if !mods.shift_key() {
                    self.state.next_focused_widget = self.widget_from_focus_chain(true);
                } else {
                    self.state.next_focused_widget = self.widget_from_focus_chain(false);
                }
            }
        }

        self.post_event_processing(&mut root_state);
        self.get_root_widget().debug_validate(false);

        handled
    }

    // --- MARK: ACCESS_EVENT ---
    pub fn root_on_access_event(&mut self, event: ActionRequest) {
        let mut root_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");

        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();

        let mut ctx = EventCtx {
            global_state: &mut self.state,
            widget_state: &mut root_state,
            widget_state_children: root_state_token,
            widget_children: root_widget_token,
            is_handled: false,
            request_pan_to_child: None,
        };

        let Ok(id) = event.target.0.try_into() else {
            warn!("Received ActionRequest with id 0. This shouldn't be possible.");
            return;
        };
        let event = AccessEvent {
            target: WidgetId(id),
            action: event.action,
            data: event.data,
        };

        {
            ctx.global_state
                .debug_logger
                .push_important_span(&format!("ACCESS_EVENT {}", event.short_name()));
            let _span = info_span!("access_event").entered();
            debug!("Running ON_ACCESS_EVENT pass with {}", event.short_name());

            self.root.on_access_event(&mut ctx, &event);

            debug!(
                focused_widget = ctx.global_state.focused_widget.map(|id| id.0),
                handled = ctx.is_handled,
                "ON_POINTER_EVENT finished",
            );
            ctx.global_state.debug_logger.pop_span();
        }

        self.post_event_processing(&mut root_state);
        self.get_root_widget().debug_validate(false);
    }

    // --- MARK: LIFECYCLE ---
    fn root_lifecycle(&mut self, event: LifeCycle) {
        let mut root_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");

        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();
        let mut ctx = LifeCycleCtx {
            global_state: &mut self.state,
            widget_state: &mut root_state,
            widget_state_children: root_state_token,
            widget_children: root_widget_token,
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
        self.post_event_processing(&mut root_state);
    }

    // --- MARK: LAYOUT ---
    pub(crate) fn root_layout(&mut self) {
        let mut root_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let size = self.get_kurbo_size();
        let mouse_pos = self.last_mouse_pos.map(|pos| (pos.x, pos.y).into());
        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();
        let mut layout_ctx = LayoutCtx {
            global_state: &mut self.state,
            widget_state: &mut root_state,
            widget_state_children: root_state_token,
            widget_children: root_widget_token,
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
        self.post_event_processing(&mut root_state);
    }

    // --- MARK: PAINT ---
    fn root_paint(&mut self) -> Scene {
        // TODO - Handle Xilem's VIEW_CONTEXT_CHANGED

        let mut root_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();
        let mut ctx = PaintCtx {
            global_state: &mut self.state,
            widget_state: &mut root_state,
            widget_state_children: root_state_token,
            widget_children: root_widget_token,
            depth: 0,
            debug_paint: false,
            debug_widget: false,
        };

        let mut scene = Scene::new();
        {
            let _span = info_span!("paint").entered();
            self.root.paint(&mut ctx, &mut scene);
        }

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

    // --- MARK: ACCESSIBILITY ---
    // TODO - Integrate in unit tests?
    fn root_accessibility(&mut self) -> TreeUpdate {
        let mut tree_update = TreeUpdate {
            nodes: vec![],
            tree: None,
            focus: self.state.focused_widget.unwrap_or(self.root.id()).into(),
        };
        let mut root_state =
            WidgetState::new(self.root.id(), Some(self.get_kurbo_size()), "<root>");
        let root_state_token = self.widget_arena.widget_states.root_token_mut();
        let root_widget_token = self.widget_arena.widgets.root_token_mut();
        let mut ctx = AccessCtx {
            global_state: &mut self.state,
            widget_state: &mut root_state,
            widget_state_children: root_state_token,
            widget_children: root_widget_token,
            tree_update: &mut tree_update,
            current_node: NodeBuilder::default(),
            rebuild_all: self.rebuild_access_tree,
            scale_factor: self.scale_factor,
        };

        {
            let _span = info_span!("accessibility").entered();
            if self.rebuild_access_tree {
                debug!("Running ACCESSIBILITY pass with rebuild_all");
            }
            self.root.accessibility(&mut ctx);
            self.rebuild_access_tree = false;
        }

        if true {
            tree_update.tree = Some(Tree {
                root: self.root.id().into(),
                app_name: None,
                toolkit_name: Some("Masonry".to_string()),
                toolkit_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            });
        }

        tree_update
    }

    fn get_kurbo_size(&self) -> kurbo::Size {
        let size = self.size.to_logical(self.scale_factor);
        kurbo::Size::new(size.width, size.height)
    }

    // --- MARK: POST-EVENT ---
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

        if self.root_state().needs_window_origin && !self.root_state().needs_layout {
            let event = LifeCycle::Internal(InternalLifeCycle::ParentWindowOrigin {
                mouse_pos: self.last_mouse_pos,
            });
            self.root_lifecycle(event);
        }

        // Update the disabled state if necessary
        // Always do this before updating the focus-chain
        if self.root_state().tree_disabled_changed() {
            let event = LifeCycle::Internal(InternalLifeCycle::RouteDisabledChanged);
            self.root_lifecycle(event);
        }

        // Update the focus-chain if necessary
        // Always do this before sending focus change, since this event updates the focus chain.
        if self.root_state().update_focus_chain {
            let event = LifeCycle::BuildFocusChain;
            self.root_lifecycle(event);
        }

        self.update_focus();

        if self.root_state().request_anim {
            self.state
                .signal_queue
                .push_back(RenderRootSignal::RequestAnimFrame);
        }

        // We request a redraw if either the render tree or the accessibility
        // tree needs to be rebuilt. Usually both happen at the same time.
        // A redraw will trigger a rebuild of the accessibility tree.
        if self.root_state().needs_paint || self.root_state().needs_accessibility_update {
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

    fn update_focus(&mut self) {
        let old = self.state.focused_widget;
        let new = self.state.next_focused_widget;

        // TODO
        // Skip change if requested widget is disabled

        // Only send RouteFocusChanged in case there's actual change
        if old != new {
            let event = LifeCycle::Internal(InternalLifeCycle::RouteFocusChanged { old, new });
            self.state.focused_widget = new;
            self.root_lifecycle(event);

            // TODO: discriminate between text focus, and non-text focus.
            self.state.signal_queue.push_back(if new.is_some() {
                RenderRootSignal::StartIme
            } else {
                RenderRootSignal::EndIme
            });
        }
    }

    fn widget_from_focus_chain(&mut self, forward: bool) -> Option<WidgetId> {
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
    pub(crate) fn focus_chain(&mut self) -> &[WidgetId] {
        &self.root_state().focus_chain
    }
}

/*
TODO:
- Invalidation regions
- Timer handling
- prepare_paint
- Focus-related stuff
*/
