// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, VecDeque};

use accesskit::{ActionRequest, TreeUpdate};
use parley::fontique::{self, Collection, CollectionOptions};
use parley::{FontContext, LayoutContext};
use tracing::{info_span, warn};
use vello::kurbo::{self, Rect};
use vello::Scene;
use winit::window::ResizeDirection;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::debug_logger::DebugLogger;
use crate::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use crate::event::{PointerEvent, TextEvent, WindowEvent};
use crate::passes::accessibility::run_accessibility_pass;
use crate::passes::anim::run_update_anim_pass;
use crate::passes::compose::run_compose_pass;
use crate::passes::event::{
    run_on_access_event_pass, run_on_pointer_event_pass, run_on_text_event_pass,
};
use crate::passes::layout::run_layout_pass;
use crate::passes::mutate::{mutate_widget, run_mutate_pass};
use crate::passes::paint::run_paint_pass;
use crate::passes::update::{
    run_update_disabled_pass, run_update_focus_chain_pass, run_update_focus_pass,
    run_update_pointer_pass, run_update_scroll_pass, run_update_stashed_pass,
    run_update_widget_tree_pass,
};
use crate::passes::{recurse_on_children, PassTracing};
use crate::text::BrushIndex;
use crate::tree_arena::{ArenaMut, TreeArena};
use crate::widget::{WidgetArena, WidgetMut, WidgetRef, WidgetState};
use crate::{AccessEvent, Action, CursorIcon, Handled, QueryCtx, Widget, WidgetId, WidgetPod};

// --- MARK: STRUCTS ---

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
    pub(crate) global_state: RenderRootState,
    // TODO - Add "access_tree_active" to detect when you don't need to update the
    // access tree
    pub(crate) rebuild_access_tree: bool,
    pub(crate) widget_arena: WidgetArena,
}

// TODO - Document these fields.
pub(crate) struct RenderRootState {
    pub(crate) debug_logger: DebugLogger,
    pub(crate) signal_queue: VecDeque<RenderRootSignal>,
    pub(crate) focused_widget: Option<WidgetId>,
    pub(crate) focused_path: Vec<WidgetId>,
    pub(crate) next_focused_widget: Option<WidgetId>,
    pub(crate) scroll_request_targets: Vec<(WidgetId, Rect)>,
    pub(crate) hovered_path: Vec<WidgetId>,
    pub(crate) pointer_capture_target: Option<WidgetId>,
    pub(crate) cursor_icon: CursorIcon,
    pub(crate) font_context: FontContext,
    pub(crate) text_layout_context: LayoutContext<BrushIndex>,
    pub(crate) mutate_callbacks: Vec<MutateCallback>,
    pub(crate) is_ime_active: bool,
    pub(crate) scenes: HashMap<WidgetId, Scene>,
    /// Whether data set in the pointer pass has been invalidated.
    pub(crate) needs_pointer_pass: bool,
    pub(crate) trace: PassTracing,
}

pub(crate) struct MutateCallback {
    pub(crate) id: WidgetId,
    pub(crate) callback: Box<dyn FnOnce(WidgetMut<'_, Box<dyn Widget>>)>,
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

pub struct RenderRootOptions {
    pub use_system_fonts: bool,
    pub size_policy: WindowSizePolicy,
    pub scale_factor: f64,

    /// Add a font from its raw data for use in tests.
    /// The font is added to the fallback chain for Latin scripts.
    /// This is expected to be used with `use_system_fonts = false`
    /// to ensure rendering is consistent cross-platform.
    ///
    /// We expect to develop a much more fully-featured font API in the future, but
    /// this is necessary for our testing of Masonry.
    pub test_font: Option<Vec<u8>>,
}

pub enum RenderRootSignal {
    Action(Action, WidgetId),
    StartIme,
    EndIme,
    ImeMoved(LogicalPosition<f64>, LogicalSize<f64>),
    RequestRedraw,
    RequestAnimFrame,
    TakeFocus,
    SetCursor(CursorIcon),
    SetSize(PhysicalSize<u32>),
    SetTitle(String),
    DragWindow,
    DragResizeWindow(ResizeDirection),
    ToggleMaximized,
    Minimize,
    Exit,
    ShowWindowMenu(LogicalPosition<f64>),
}

impl RenderRoot {
    pub fn new(
        root_widget: impl Widget,
        RenderRootOptions {
            use_system_fonts,
            size_policy,
            scale_factor,
            test_font,
        }: RenderRootOptions,
    ) -> Self {
        let mut root = RenderRoot {
            root: WidgetPod::new(root_widget).boxed(),
            size_policy,
            size: PhysicalSize::new(0, 0),
            scale_factor,
            last_anim: None,
            last_mouse_pos: None,
            cursor_icon: CursorIcon::Default,
            global_state: RenderRootState {
                debug_logger: DebugLogger::new(false),
                signal_queue: VecDeque::new(),
                focused_widget: None,
                focused_path: Vec::new(),
                next_focused_widget: None,
                scroll_request_targets: Vec::new(),
                hovered_path: Vec::new(),
                pointer_capture_target: None,
                cursor_icon: CursorIcon::Default,
                font_context: FontContext {
                    collection: Collection::new(CollectionOptions {
                        system_fonts: use_system_fonts,
                        ..Default::default()
                    }),
                    source_cache: Default::default(),
                },
                text_layout_context: LayoutContext::new(),
                mutate_callbacks: Vec::new(),
                is_ime_active: false,
                scenes: HashMap::new(),
                needs_pointer_pass: false,
                trace: PassTracing::from_env(),
            },
            widget_arena: WidgetArena {
                widgets: TreeArena::new(),
                widget_states: TreeArena::new(),
            },
            rebuild_access_tree: true,
        };

        if let Some(test_font_data) = test_font {
            let families = root.register_fonts(test_font_data);
            // Make sure that all of these fonts are in the fallback chain for the Latin script.
            // <https://en.wikipedia.org/wiki/Script_(Unicode)#Latn>
            root.global_state
                .font_context
                .collection
                .append_fallbacks(*b"Latn", families.iter().map(|(family, _)| *family));
        }

        // We run a set of passes to initialize the widget tree
        root.run_rewrite_passes();

        root
    }

    pub(crate) fn root_state(&mut self) -> &mut WidgetState {
        self.widget_arena
            .widget_states
            .root_token_mut()
            .into_child_mut(self.root.id())
            .expect("root widget not in widget tree")
            .item
    }

    // --- MARK: WINDOW_EVENT ---
    pub fn handle_window_event(&mut self, event: WindowEvent) -> Handled {
        match event {
            WindowEvent::Rescale(scale_factor) => {
                self.scale_factor = scale_factor;
                self.request_render_all();
                Handled::Yes
            }
            WindowEvent::Resize(size) => {
                self.size = size;
                self.root_state().request_layout = true;
                self.root_state().needs_layout = true;
                self.run_rewrite_passes();
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

                run_update_anim_pass(self, elapsed_ns);
                self.run_rewrite_passes();

                // If this animation will continue, store the time.
                // If a new animation starts, then it will have zero reported elapsed time.
                let animation_continues = self.root_state().needs_anim;
                self.last_anim = animation_continues.then_some(now);

                Handled::Yes
            }
            WindowEvent::RebuildAccessTree => {
                self.rebuild_access_tree = true;
                self.global_state
                    .emit_signal(RenderRootSignal::RequestRedraw);
                Handled::Yes
            }
        }
    }

    // --- MARK: PUB FUNCTIONS ---
    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> Handled {
        let _span = info_span!("pointer_event");
        let handled = run_on_pointer_event_pass(self, &event);
        run_update_pointer_pass(self);
        self.run_rewrite_passes();

        handled
    }

    pub fn handle_text_event(&mut self, event: TextEvent) -> Handled {
        let _span = info_span!("text_event");
        let handled = run_on_text_event_pass(self, &event);
        run_update_focus_pass(self);
        self.run_rewrite_passes();

        handled
    }

    pub fn handle_access_event(&mut self, event: ActionRequest) {
        let _span = info_span!("access_event");
        let Ok(id) = event.target.0.try_into() else {
            warn!("Received ActionRequest with id 0. This shouldn't be possible.");
            return;
        };
        let event = AccessEvent {
            action: event.action,
            data: event.data,
        };

        run_on_access_event_pass(self, &event, WidgetId(id));
        self.run_rewrite_passes();
    }

    /// Registers all fonts that exist in the given data.
    ///
    /// Returns a list of pairs each containing the family identifier and fonts
    /// added to that family.
    pub fn register_fonts(
        &mut self,
        data: Vec<u8>,
    ) -> Vec<(fontique::FamilyId, Vec<fontique::FontInfo>)> {
        self.global_state
            .font_context
            .collection
            .register_fonts(data)
    }

    pub fn redraw(&mut self) -> (Scene, TreeUpdate) {
        if self.root_state().needs_layout {
            // TODO - Rewrite more clearly after run_rewrite_passes is rewritten
            self.run_rewrite_passes();
        }
        if self.root_state().needs_layout {
            warn!("Widget requested layout during layout pass");
            self.global_state
                .emit_signal(RenderRootSignal::RequestRedraw);
        }

        // TODO - Handle invalidation regions
        // TODO - Improve caching of scenes.
        (
            run_paint_pass(self),
            run_accessibility_pass(self, self.scale_factor),
        )
    }

    pub fn pop_signal(&mut self) -> Option<RenderRootSignal> {
        self.global_state.signal_queue.pop_front()
    }

    pub fn pop_signal_matching(
        &mut self,
        predicate: impl Fn(&RenderRootSignal) -> bool,
    ) -> Option<RenderRootSignal> {
        let idx = self.global_state.signal_queue.iter().position(predicate)?;
        self.global_state.signal_queue.remove(idx)
    }

    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    // --- MARK: ACCESS WIDGETS---
    /// Get a [`WidgetRef`] to the root widget.
    pub fn get_root_widget(&self) -> WidgetRef<dyn Widget> {
        let root_state_token = self.widget_arena.widget_states.root_token();
        let root_widget_token = self.widget_arena.widgets.root_token();
        let state_ref = root_state_token
            .into_child(self.root.id())
            .expect("root widget not in widget tree");
        let widget_ref = root_widget_token
            .into_child(self.root.id())
            .expect("root widget not in widget tree");

        // Our WidgetArena stores all widgets as Box<dyn Widget>, but the "true"
        // type of our root widget is *also* Box<dyn Widget>. We downcast so we
        // don't add one more level of indirection to this.
        let widget = widget_ref
            .item
            .as_dyn_any()
            .downcast_ref::<Box<dyn Widget>>()
            .unwrap();

        let ctx = QueryCtx {
            global_state: &self.global_state,
            widget_state_children: state_ref.children,
            widget_children: widget_ref.children,
            widget_state: state_ref.item,
        };

        WidgetRef { ctx, widget }
    }

    /// Get a [`WidgetRef`] to a specific widget.
    pub fn get_widget(&self, id: WidgetId) -> Option<WidgetRef<dyn Widget>> {
        let state_ref = self.widget_arena.widget_states.find(id)?;
        let widget_ref = self
            .widget_arena
            .widgets
            .find(id)
            .expect("found state but not widget");

        // Box<dyn Widget> -> &dyn Widget
        // Without this step, the type of `WidgetRef::widget` would be
        // `&Box<dyn Widget> as &dyn Widget`, which would be an additional layer
        // of indirection.
        let widget = widget_ref.item;
        let widget: &dyn Widget = &**widget;
        let ctx = QueryCtx {
            global_state: &self.global_state,
            widget_state_children: state_ref.children,
            widget_children: widget_ref.children,
            widget_state: state_ref.item,
        };
        Some(WidgetRef { ctx, widget })
    }

    /// Get a [`WidgetMut`] to the root widget.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    pub fn edit_root_widget<R>(
        &mut self,
        f: impl FnOnce(WidgetMut<'_, Box<dyn Widget>>) -> R,
    ) -> R {
        let res = mutate_widget(self, self.root.id(), |mut widget_mut| {
            // Our WidgetArena stores all widgets as Box<dyn Widget>, but the "true"
            // type of our root widget is *also* Box<dyn Widget>. We downcast so we
            // don't add one more level of indirection to this.
            let widget = widget_mut
                .widget
                .as_mut_dyn_any()
                .downcast_mut::<Box<dyn Widget>>()
                .unwrap();
            let widget_mut = WidgetMut {
                ctx: widget_mut.ctx.reborrow_mut(),
                widget,
            };
            f(widget_mut)
        });

        self.run_rewrite_passes();

        res
    }

    /// Get a [`WidgetMut`] to a specific widget.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    pub fn edit_widget<R>(
        &mut self,
        id: WidgetId,
        f: impl FnOnce(WidgetMut<'_, Box<dyn Widget>>) -> R,
    ) -> R {
        let res = mutate_widget(self, id, f);

        self.run_rewrite_passes();

        res
    }

    pub(crate) fn get_kurbo_size(&self) -> kurbo::Size {
        let size = self.size.to_logical(self.scale_factor);
        kurbo::Size::new(size.width, size.height)
    }

    // --- MARK: REWRITE PASSES ---
    /// Run all rewrite passes on widget tree.
    ///
    /// Rewrite passes are passes which occur after external events, and
    /// update flags and internal values to a consistent state.
    ///
    /// See Pass Spec RFC for details. (TODO - Link to doc instead.)
    pub(crate) fn run_rewrite_passes(&mut self) {
        const REWRITE_PASSES_MAX: usize = 4;

        for _ in 0..REWRITE_PASSES_MAX {
            // Note: this code doesn't do any short-circuiting, because each pass is
            // expected to have its own early exits.
            // Calling a run_xxx_pass (or root_xxx) should always be very fast if
            // the pass doesn't need to do anything.

            run_mutate_pass(self);
            run_update_widget_tree_pass(self);
            run_update_disabled_pass(self);
            run_update_stashed_pass(self);
            run_update_focus_chain_pass(self);
            run_update_focus_pass(self);
            run_layout_pass(self);
            run_update_scroll_pass(self);
            run_compose_pass(self);
            run_update_pointer_pass(self);

            if !self.root_state().needs_rewrite_passes()
                && !self.global_state.needs_rewrite_passes()
            {
                break;
            }
        }

        if self.root_state().needs_rewrite_passes() || self.global_state.needs_rewrite_passes() {
            warn!("All rewrite passes have run {REWRITE_PASSES_MAX} times, but invalidations are still set");
            // To avoid an infinite loop, we delay re-running the passes until the next frame.
            self.global_state
                .emit_signal(RenderRootSignal::RequestRedraw);
        }

        if self.root_state().needs_anim {
            self.global_state
                .emit_signal(RenderRootSignal::RequestAnimFrame);
        }

        // We request a redraw if either the render tree or the accessibility
        // tree needs to be rebuilt. Usually both happen at the same time.
        // A redraw will trigger a rebuild of the accessibility tree.
        // TODO - We assume that a relayout will trigger a repaint
        if self.root_state().needs_paint || self.root_state().needs_accessibility {
            self.global_state
                .emit_signal(RenderRootSignal::RequestRedraw);
        }
    }

    pub(crate) fn request_render_all(&mut self) {
        fn request_render_all_in(
            mut widget: ArenaMut<'_, Box<dyn Widget>>,
            state: ArenaMut<'_, WidgetState>,
        ) {
            state.item.needs_paint = true;
            state.item.needs_accessibility = true;
            state.item.request_paint = true;
            state.item.request_accessibility = true;

            let id = state.item.id;
            recurse_on_children(
                id,
                widget.reborrow_mut(),
                state.children,
                |widget, mut state| {
                    request_render_all_in(widget, state.reborrow_mut());
                },
            );
        }

        let (root_widget, mut root_state) = self.widget_arena.get_pair_mut(self.root.id());
        request_render_all_in(root_widget, root_state.reborrow_mut());
        self.global_state
            .emit_signal(RenderRootSignal::RequestRedraw);
    }

    // Checks whether the given id points to a widget that is "interactive".
    // i.e. not disabled or stashed.
    // Only interactive widgets can have text focus or pointer capture.
    pub(crate) fn is_still_interactive(&self, id: WidgetId) -> bool {
        let Some(state) = self.widget_arena.widget_states.find(id) else {
            return false;
        };

        !state.item.is_stashed && !state.item.is_disabled
    }

    pub(crate) fn widget_from_focus_chain(&mut self, forward: bool) -> Option<WidgetId> {
        let focused_widget = self.global_state.focused_widget;
        let focused_idx = focused_widget.and_then(|focused_widget| {
            self.focus_chain()
                .iter()
                // Find where the focused widget is in the focus chain
                .position(|id| id == &focused_widget)
        });

        if let Some(idx) = focused_idx {
            // Return the id that's next to it in the focus chain
            let len = self.focus_chain().len();
            let new_idx = if forward {
                (idx + 1) % len
            } else {
                (idx + len - 1) % len
            };
            Some(self.focus_chain()[new_idx])
        } else {
            // If no widget is currently focused or the
            // currently focused widget isn't in the focus chain,
            // then we'll just return the first/last entry of the chain, if any.
            if forward {
                self.focus_chain().first().copied()
            } else {
                self.focus_chain().last().copied()
            }
        }
    }

    // TODO - Store in RenderRootState
    pub(crate) fn focus_chain(&mut self) -> &[WidgetId] {
        &self.root_state().focus_chain
    }

    #[allow(dead_code)]
    pub(crate) fn needs_rewrite_passes(&mut self) -> bool {
        self.root_state().needs_rewrite_passes() || self.global_state.focus_changed()
    }
}

impl RenderRootState {
    /// Send a signal to the runner of this app, which allows global actions to be triggered by a widget.
    pub(crate) fn emit_signal(&mut self, signal: RenderRootSignal) {
        self.signal_queue.push_back(signal);
    }

    pub(crate) fn focus_changed(&self) -> bool {
        self.focused_widget != self.next_focused_widget
    }

    pub(crate) fn is_focused(&self, id: WidgetId) -> bool {
        self.focused_widget == Some(id)
    }

    pub(crate) fn needs_rewrite_passes(&self) -> bool {
        self.needs_pointer_pass || self.focused_widget != self.next_focused_widget
    }
}

impl RenderRootSignal {
    pub(crate) fn new_ime_moved_signal(area: Rect) -> Self {
        RenderRootSignal::ImeMoved(
            LogicalPosition {
                x: area.origin().x,
                y: area.origin().y + area.size().height,
            },
            LogicalSize {
                width: area.size().width,
                height: area.size().height,
            },
        )
    }
}
