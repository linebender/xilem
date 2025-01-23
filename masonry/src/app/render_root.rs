// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::collections::VecDeque;

use accesskit::ActionRequest;
use accesskit::TreeUpdate;
use parley::fontique::Collection;
use parley::fontique::CollectionOptions;
use parley::fontique::{self};
use parley::FontContext;
use parley::LayoutContext;
use tracing::info_span;
use tracing::warn;
use tree_arena::ArenaMut;
use tree_arena::TreeArena;
use vello::kurbo::Rect;
use vello::kurbo::{self};
use vello::Scene;
use winit::window::ResizeDirection;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::core::AccessEvent;
use crate::core::Action;
use crate::core::BrushIndex;
use crate::core::PointerEvent;
use crate::core::QueryCtx;
use crate::core::TextEvent;
use crate::core::Widget;
use crate::core::WidgetArena;
use crate::core::WidgetId;
use crate::core::WidgetMut;
use crate::core::WidgetPod;
use crate::core::WidgetRef;
use crate::core::WidgetState;
use crate::core::WindowEvent;
use crate::dpi::LogicalPosition;
use crate::dpi::LogicalSize;
use crate::dpi::PhysicalSize;
use crate::passes::accessibility::run_accessibility_pass;
use crate::passes::anim::run_update_anim_pass;
use crate::passes::compose::run_compose_pass;
use crate::passes::event::run_on_access_event_pass;
use crate::passes::event::run_on_pointer_event_pass;
use crate::passes::event::run_on_text_event_pass;
use crate::passes::layout::run_layout_pass;
use crate::passes::mutate::mutate_widget;
use crate::passes::mutate::run_mutate_pass;
use crate::passes::paint::run_paint_pass;
use crate::passes::recurse_on_children;
use crate::passes::update::run_update_disabled_pass;
use crate::passes::update::run_update_focus_chain_pass;
use crate::passes::update::run_update_focus_pass;
use crate::passes::update::run_update_pointer_pass;
use crate::passes::update::run_update_scroll_pass;
use crate::passes::update::run_update_stashed_pass;
use crate::passes::update::run_update_widget_tree_pass;
use crate::passes::PassTracing;
use crate::Handled;
use cursor_icon::CursorIcon;

/// We ensure that any valid initial IME area is sent to the platform by storing an invalid initial
/// IME area as the `last_sent_ime_area`.
const INVALID_IME_AREA: Rect = Rect::new(f64::NAN, f64::NAN, f64::NAN, f64::NAN);

// --- MARK: STRUCTS ---

/// The composition root of Masonry.
///
/// This is the entry point for all user events, and the source of all signals to be sent to
/// winit or similar event loop runners, as well as 2D scenes and accessibility information.
///
/// This is also the type that owns the widget tree.
pub struct RenderRoot {
    /// Root of the widget tree.
    pub(crate) root: WidgetPod<dyn Widget>,

    /// Whether the window size should be determined by the content or the user.
    pub(crate) size_policy: WindowSizePolicy,

    /// Current size of the window.
    pub(crate) size: PhysicalSize<u32>,

    /// DPI scale factor.
    ///
    /// Kurbo coordinates are assumed to be in logical pixels
    pub(crate) scale_factor: f64,

    /// Is `Some` if the most recently displayed frame was an animation frame.
    pub(crate) last_anim: Option<Instant>,

    /// Last mouse position. Updated by `on_pointer_event` pass, used by other passes.
    pub(crate) last_mouse_pos: Option<LogicalPosition<f64>>,

    /// State passed to context types.
    pub(crate) global_state: RenderRootState,

    /// Whether the next accessibility pass should rebuild the entire access tree.
    ///
    /// TODO - Add `access_tree_active` to detect when you don't need to update the
    // access tree
    pub(crate) rebuild_access_tree: bool,

    /// The widget tree; stores widgets and their states.
    pub(crate) widget_arena: WidgetArena,
    pub(crate) debug_paint: bool,
}

/// State shared between passes.
pub(crate) struct RenderRootState {
    /// Queue of signals to be processed by the event loop.
    pub(crate) signal_queue: VecDeque<RenderRootSignal>,

    /// Currently focused widget.
    pub(crate) focused_widget: Option<WidgetId>,

    /// List of ancestors of the currently focused widget.
    pub(crate) focused_path: Vec<WidgetId>,

    /// Widget that will be focused once the `update_focus` pass is run.
    pub(crate) next_focused_widget: Option<WidgetId>,

    /// Most recently clicked widget.
    ///
    /// This is used to pick the focused widget on Tab events.
    pub(crate) most_recently_clicked_widget: Option<WidgetId>,

    /// Whether the window is focused.
    pub(crate) window_focused: bool,

    /// Widgets that have requested to be scrolled into view.
    pub(crate) scroll_request_targets: Vec<(WidgetId, Rect)>,

    /// List of ancestors of the currently hovered widget.
    pub(crate) hovered_path: Vec<WidgetId>,

    /// Widget that currently has pointer capture.
    pub(crate) pointer_capture_target: Option<WidgetId>,

    /// Current cursor icon.
    pub(crate) cursor_icon: CursorIcon,

    /// Cache for Parley font data.
    pub(crate) font_context: FontContext,

    /// Cache for Parley text layout data.
    pub(crate) text_layout_context: LayoutContext<BrushIndex>,

    /// List of callbacks that will run in the next `mutate` pass.
    pub(crate) mutate_callbacks: Vec<MutateCallback>,

    /// Whether an IME session is active.
    pub(crate) is_ime_active: bool,

    /// The cursor area last sent to the platform.
    pub(crate) last_sent_ime_area: Rect,

    /// Scene cache for the widget tree.
    pub(crate) scenes: HashMap<WidgetId, Scene>,

    /// Whether data set in the pointer pass has been invalidated.
    pub(crate) needs_pointer_pass: bool,

    /// Pass tracing configuration, used to skip tracing to limit overhead.
    pub(crate) trace: PassTracing,
    pub(crate) inspector_state: InspectorState,
}

pub(crate) struct MutateCallback {
    pub(crate) id: WidgetId,
    pub(crate) callback: Box<dyn FnOnce(WidgetMut<'_, dyn Widget>)>,
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

/// Options for creating a [`RenderRoot`].
pub struct RenderRootOptions {
    /// If true, `fontique` will provide access to system fonts
    /// using platform-specific APIs.
    pub use_system_fonts: bool,

    /// Defines how the window size should be determined.
    pub size_policy: WindowSizePolicy,

    /// The scale factor to use for rendering.
    ///
    /// Useful for high-DPI displays.
    ///
    /// `1.0` is a sensible default.
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

/// Objects emitted by the [`RenderRoot`] to signal that something has changed or require external actions.
pub enum RenderRootSignal {
    /// A widget has emitted an action.
    Action(Action, WidgetId),
    /// An IME session has been started.
    StartIme,
    /// The IME session has ended.
    EndIme,
    /// The IME area has been moved.
    ImeMoved(LogicalPosition<f64>, LogicalSize<f64>),
    /// The window needs to be redrawn.
    RequestRedraw,
    /// The window should be redrawn for an animation frame. Currently this isn't really different from `RequestRedraw`.
    RequestAnimFrame,
    /// The window should take focus.
    TakeFocus,
    /// The mouse icon has changed.
    SetCursor(CursorIcon),
    /// The window size has changed.
    SetSize(PhysicalSize<u32>),
    /// The window title has changed.
    SetTitle(String),
    /// The window is being dragged.
    DragWindow,
    /// The window is being resized.
    DragResizeWindow(ResizeDirection),
    /// The window is being maximized.
    ToggleMaximized,
    /// The window is being minimized.
    Minimize,
    /// The window is being closed.
    Exit,
    /// The window menu is being shown.
    ShowWindowMenu(LogicalPosition<f64>),
    /// The widget picker has selected this widget.
    WidgetSelectedInInspector(WidgetId),
}

/// State of the widget inspector. Useful for debugging.
///
/// Widget inspector is WIP. It should get its own standalone documentation.
pub(crate) struct InspectorState {
    pub(crate) is_picking_widget: bool,
    pub(crate) hovered_widget: Option<WidgetId>,
}

impl RenderRoot {
    /// Create a new `RenderRoot` with the given options.
    ///
    /// Note that this doesn't create a window or start the event loop.
    ///
    /// See [`crate::app::run`] for that.
    pub fn new(root_widget: impl Widget, options: RenderRootOptions) -> Self {
        let RenderRootOptions {
            use_system_fonts,
            size_policy,
            scale_factor,
            test_font,
        } = options;
        let debug_paint = std::env::var("MASONRY_DEBUG_PAINT").is_ok_and(|it| !it.is_empty());

        let mut root = Self {
            root: WidgetPod::new(root_widget).erased(),
            size_policy,
            size: PhysicalSize::new(0, 0),
            scale_factor,
            last_anim: None,
            last_mouse_pos: None,
            global_state: RenderRootState {
                signal_queue: VecDeque::new(),
                focused_widget: None,
                focused_path: Vec::new(),
                next_focused_widget: None,
                most_recently_clicked_widget: None,
                window_focused: true,
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
                last_sent_ime_area: INVALID_IME_AREA,
                scenes: HashMap::new(),
                needs_pointer_pass: false,
                trace: PassTracing::from_env(),
                inspector_state: InspectorState {
                    is_picking_widget: false,
                    hovered_widget: None,
                },
            },
            widget_arena: WidgetArena {
                widgets: TreeArena::new(),
                states: TreeArena::new(),
            },
            rebuild_access_tree: true,
            debug_paint,
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

    pub(crate) fn root_state(&self) -> &WidgetState {
        self.widget_arena
            .states
            .roots()
            .into_item(self.root.id())
            .expect("root widget not in widget tree")
            .item
    }

    pub(crate) fn root_state_mut(&mut self) -> &mut WidgetState {
        self.widget_arena
            .states
            .roots_mut()
            .into_item_mut(self.root.id())
            .expect("root widget not in widget tree")
            .item
    }

    // --- MARK: WINDOW_EVENT ---
    /// Handle a window event.
    pub fn handle_window_event(&mut self, event: WindowEvent) -> Handled {
        match event {
            WindowEvent::Rescale(scale_factor) => {
                self.scale_factor = scale_factor;
                self.request_render_all();
                Handled::Yes
            }
            WindowEvent::Resize(size) => {
                self.size = size;
                self.root_state_mut().request_layout = true;
                self.root_state_mut().needs_layout = true;
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
    /// Handle a pointer event.
    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> Handled {
        let _span = info_span!("pointer_event");
        let handled = run_on_pointer_event_pass(self, &event);
        run_update_pointer_pass(self);
        self.run_rewrite_passes();

        handled
    }

    /// Handle a text event.
    pub fn handle_text_event(&mut self, event: TextEvent) -> Handled {
        let _span = info_span!("text_event");
        let handled = run_on_text_event_pass(self, &event);
        run_update_focus_pass(self);

        if matches!(event, TextEvent::Ime(winit::event::Ime::Enabled)) {
            // Reset the last sent IME area, as the platform reset the IME state and may have
            // forgotten it.
            self.global_state.last_sent_ime_area = INVALID_IME_AREA;
        }
        self.run_rewrite_passes();

        handled
    }

    /// Handle an accesskit event.
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

    /// Redraw the window.
    ///
    /// Returns an update to the accessibility tree and a Vello scene representing
    /// the widget tree's current state.
    pub fn redraw(&mut self) -> (Scene, TreeUpdate) {
        self.run_rewrite_passes();

        // TODO - Handle invalidation regions
        let scene = run_paint_pass(self);
        let tree_update = run_accessibility_pass(self, self.scale_factor);
        (scene, tree_update)
    }

    /// Pop the oldest signal from the queue.
    pub fn pop_signal(&mut self) -> Option<RenderRootSignal> {
        self.global_state.signal_queue.pop_front()
    }

    /// Pop the oldest signal from the queue that matches the predicate.
    ///
    /// Doesn't affect other signals.
    ///
    /// Note that you should still use [`Self::pop_signal`] to avoid letting the queue
    /// grow indefinitely.
    pub fn pop_signal_matching(
        &mut self,
        predicate: impl Fn(&RenderRootSignal) -> bool,
    ) -> Option<RenderRootSignal> {
        let idx = self.global_state.signal_queue.iter().position(predicate)?;
        self.global_state.signal_queue.remove(idx)
    }

    /// Get the current icon that the mouse should display.
    pub fn cursor_icon(&self) -> CursorIcon {
        self.global_state.cursor_icon
    }

    // --- MARK: ACCESS WIDGETS---
    /// Get a [`WidgetRef`] to the root widget.
    pub fn get_root_widget(&self) -> WidgetRef<dyn Widget> {
        let root_state_token = self.widget_arena.states.roots();
        let root_widget_token = self.widget_arena.widgets.roots();
        let state_ref = root_state_token
            .into_item(self.root.id())
            .expect("root widget not in widget tree");
        let widget_ref = root_widget_token
            .into_item(self.root.id())
            .expect("root widget not in widget tree");

        let widget = &**widget_ref.item;
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
        let state_ref = self.widget_arena.states.find(id)?;
        let widget_ref = self
            .widget_arena
            .widgets
            .find(id)
            .expect("found state but not widget");

        let widget = &**widget_ref.item;
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
    pub fn edit_root_widget<R>(&mut self, f: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R) -> R {
        let res = mutate_widget(self, self.root.id(), f);

        self.run_rewrite_passes();

        res
    }

    /// Get a [`WidgetMut`] to a specific widget.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    pub fn edit_widget<R>(
        &mut self,
        id: WidgetId,
        f: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R,
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
    /// See the [passes documentation](../doc/05_pass_system.md) for details.
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
        if self.root_state().needs_paint || self.root_state().needs_accessibility {
            self.global_state
                .emit_signal(RenderRootSignal::RequestRedraw);
        }

        if self.global_state.is_ime_active {
            let widget = self
                .global_state
                .focused_widget
                .expect("IME is active without a focused widget");
            let ime_area = self.widget_arena.get_state(widget).item.get_ime_area();
            // Certain desktop environments (primarily KDE on Wayland) re-synchronise IME state
            // with the client (this app) in response to the safe area changing.
            // Our handling of that ultimately results in us sending the safe area again,
            // which causes an infinite loop.
            // We break that loop by not re-sending the same safe area again.
            if self.global_state.last_sent_ime_area != ime_area {
                self.global_state.last_sent_ime_area = ime_area;
                self.global_state
                    .emit_signal(RenderRootSignal::new_ime_moved_signal(ime_area));
            }
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

    /// Checks whether the given id points to a widget that is "interactive".
    /// i.e. not disabled or stashed.
    /// Only interactive widgets can have text focus or pointer capture.
    pub(crate) fn is_still_interactive(&self, id: WidgetId) -> bool {
        let Some(state) = self.widget_arena.states.find(id) else {
            return false;
        };

        !state.item.is_stashed && !state.item.is_disabled
    }

    pub(crate) fn widget_from_focus_chain(&mut self, forward: bool) -> Option<WidgetId> {
        let focused_widget = self
            .global_state
            .focused_widget
            .or(self.global_state.most_recently_clicked_widget);
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

    pub(crate) fn needs_rewrite_passes(&self) -> bool {
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

    #[expect(
        dead_code,
        reason = "no longer used, but may be useful again in the future"
    )]
    pub(crate) fn is_focused(&self, id: WidgetId) -> bool {
        self.focused_widget == Some(id)
    }

    pub(crate) fn needs_rewrite_passes(&self) -> bool {
        self.needs_pointer_pass || self.focused_widget != self.next_focused_widget
    }
}

impl RenderRootSignal {
    pub(crate) fn new_ime_moved_signal(area: Rect) -> Self {
        Self::ImeMoved(
            LogicalPosition {
                x: area.origin().x,
                y: area.origin().y,
            },
            LogicalSize {
                width: area.size().width,
                height: area.size().height,
            },
        )
    }
}
