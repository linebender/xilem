// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use accesskit::{ActionRequest, NodeId, TreeUpdate};
use dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use parley::fontique::{Blob, Collection, CollectionOptions, FamilyId, FontInfo, SourceCache};
use parley::{FontContext, LayoutContext};
use tracing::{debug, info_span, warn};
use tree_arena::{ArenaMut, TreeArena};
use vello::Scene;
use vello::kurbo::{Point, Rect, Size};

use crate::app::layer_stack::LayerStack;
use crate::core::{
    AccessCtx, AccessEvent, BrushIndex, CursorIcon, DefaultProperties, ErasedAction, FromDynWidget,
    Handled, Ime, LayerType, NewWidget, PointerEvent, PropertiesRef, QueryCtx, ResizeDirection,
    TextEvent, Widget, WidgetArena, WidgetArenaNode, WidgetId, WidgetMut, WidgetPod, WidgetRef,
    WidgetState, WidgetTag, WidgetTagInner, WindowEvent,
};
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
    run_update_disabled_pass, run_update_focus_pass, run_update_focusable_pass,
    run_update_fonts_pass, run_update_pointer_pass, run_update_scroll_pass,
    run_update_stashed_pass, run_update_widget_tree_pass,
};
use crate::passes::{PassTracing, recurse_on_children};
use crate::properties::Dimensions;

/// We ensure that any valid initial IME area is sent to the platform by storing an invalid initial
/// IME area as the `last_sent_ime_area`.
const INVALID_IME_AREA: Rect = Rect::new(f64::NAN, f64::NAN, f64::NAN, f64::NAN);

// --- MARK: STRUCTS

/// The composition root of Masonry.
///
/// This is the entry point for all user events, and the source of all signals to be sent to
/// winit or similar event loop runners, as well as 2D scenes and accessibility information.
///
/// This is also the type that owns the widget tree.
pub struct RenderRoot {
    /// `WidgetPod` handle for the layer stack, which holds the root widget of each layer.
    pub(crate) layer_stack: WidgetPod<LayerStack>,

    /// The accessibility pass creates a wrapper node for the app with `Role::Window`.
    ///
    /// This is the id of that node.
    pub(crate) window_node_id: NodeId,

    /// Whether the window size should be determined by the content or the user.
    pub(crate) size_policy: WindowSizePolicy,

    /// Current size of the window.
    pub(crate) size: PhysicalSize<u32>,

    /// Last mouse position. Updated by `on_pointer_event` pass, used by other passes.
    pub(crate) last_mouse_pos: Option<LogicalPosition<f64>>,

    /// Default values that properties will have if not defined per-widget.
    pub(crate) default_properties: Arc<DefaultProperties>,

    /// State passed to context types.
    pub(crate) global_state: RenderRootState,

    /// The widget tree; stores widgets and their states.
    pub(crate) widget_arena: WidgetArena,
}

/// State shared between passes.
pub(crate) struct RenderRootState {
    /// Sink for signals to be processed by the event loop.
    pub(crate) signal_sink: Box<dyn FnMut(RenderRootSignal)>,

    /// Currently focused widget.
    pub(crate) focused_widget: Option<WidgetId>,

    /// List of ancestors of the currently focused widget.
    pub(crate) focused_path: Vec<WidgetId>,

    /// Widget that will be focused once the `update_focus` pass is run.
    pub(crate) next_focused_widget: Option<WidgetId>,

    /// Most recently clicked/focused widget.
    ///
    /// This is used to pick the focused widget on Tab events.
    pub(crate) focus_anchor: Option<WidgetId>,

    /// Widget which will get text events if no widget is focused.
    pub(crate) focus_fallback: Option<WidgetId>,

    /// Whether the window is focused.
    pub(crate) window_focused: bool,

    /// Widgets that have requested to be scrolled into view.
    ///
    /// The `WidgetId` is the id of the widget that made the request.
    ///
    /// The `Rect` is the area it wants to be scrolled into view,
    /// in its border-box coordinate space.
    pub(crate) scroll_request_targets: Vec<(WidgetId, Rect)>,

    /// List of ancestors of the currently hovered widget.
    pub(crate) hovered_path: Vec<WidgetId>,

    /// List of ancestors of the currently active widget.
    pub(crate) active_path: Vec<WidgetId>,

    /// Widget that currently has pointer capture.
    pub(crate) pointer_capture_target: Option<WidgetId>,

    /// Current cursor icon.
    pub(crate) cursor_icon: CursorIcon,

    /// Cache for Parley font data.
    // TODO: move font context out of RenderRootState so that we only have it once per app
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
    pub(crate) scene_cache: HashMap<WidgetId, (Scene, Scene, Scene)>,

    pub(crate) widget_tags: HashMap<WidgetTagInner, WidgetId>,

    /// Whether data set in the pointer pass has been invalidated.
    pub(crate) needs_pointer_pass: bool,

    /// Pass tracing configuration, used to skip tracing to limit overhead.
    pub(crate) trace: PassTracing,

    /// Internal state of the widget inspector.
    pub(crate) inspector_state: InspectorState,

    /// Whether the next accessibility pass tree should be updated during `render()`.
    pub(crate) access_tree_active: bool,

    /// DPI scale factor.
    ///
    /// Kurbo coordinates are assumed to be in logical pixels
    pub(crate) scale_factor: f64,

    /// Whether to paint widget's bounding boxes and other visual helpers.
    pub(crate) debug_paint: bool,
}

pub(crate) struct MutateCallback {
    pub(crate) id: WidgetId,
    pub(crate) callback: Box<dyn FnOnce(WidgetMut<'_, dyn Widget>)>,
}

/// Defines how a window's size is determined.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum WindowSizePolicy {
    /// Measure the content to determine the window size.
    ///
    /// The window size will match the root widget's maximum preferred size.
    Content,
    /// Use the provided window size.
    #[default]
    User,
}

/// Options for creating a [`RenderRoot`].
pub struct RenderRootOptions {
    /// Default values that properties will have if not defined per-widget.
    pub default_properties: Arc<DefaultProperties>,

    /// If true, `fontique` will provide access to system fonts
    /// using platform-specific APIs.
    pub use_system_fonts: bool,

    /// Defines how the window size should be determined.
    pub size_policy: WindowSizePolicy,

    /// The size of the window.
    pub size: PhysicalSize<u32>,

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
    pub test_font: Option<Blob<u8>>,
}

/// Objects emitted by the [`RenderRoot`] to signal that something has changed or require external actions.
#[derive(Debug)]
pub enum RenderRootSignal {
    /// A widget has emitted an action.
    Action(ErasedAction, WidgetId),
    /// An IME session has been started.
    StartIme,
    /// The IME session has ended.
    EndIme,
    /// The IME area has been moved.
    ImeMoved(LogicalPosition<f64>, LogicalSize<f64>),
    /// A user interaction has sent something to the clipboard.
    ClipboardStore(String),
    /// The window needs to be redrawn.
    RequestRedraw,
    /// The window should be redrawn for an animation frame. Currently this isn't really different from `RequestRedraw`.
    RequestAnimFrame,
    /// The window should take focus.
    TakeFocus,
    /// The mouse icon should change.
    SetCursor(CursorIcon),
    /// The window should be resized.
    SetSize(PhysicalSize<u32>),
    /// The window title has changed.
    SetTitle(String),
    /// The user has started dragging the window.
    ///
    /// Masonry should send this event when the user presses the left mouse button while hovering a client-side decoration representing the window title bar or similar.
    /// The platform that receives this event should start moving the window around until the mouse button is released.
    DragWindow,
    /// The user has started drag-resizing the window.
    ///
    /// Masonry should send this event when the user presses the left mouse button while hovering a client-side decoration representing a window resize handle.
    /// The platform that receives this event should start resizing the window until the mouse button is released.
    DragResizeWindow(ResizeDirection),
    /// The window should be maximized.
    ToggleMaximized,
    /// The window should be minimized.
    Minimize,
    /// The app should terminate.
    Exit,
    /// The [window system menu] should be shown.
    ///
    /// There are no guarantees as to what the shown menu will contain, or even if a window menu will be drawn at all.
    /// In general, the menu may contain options to maximize, minimize or move the window.
    ///
    /// Some platforms may ignore this.
    ///
    /// See also [the Wayland doc for `xdg_toplevel::show_window_menu`](https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:show_window_menu).
    ///
    /// [Windows system menu]: https://en.wikipedia.org/w/index.php?title=Common_menus_in_Microsoft_Windows&oldid=1285312933#System_menu
    ShowWindowMenu(LogicalPosition<f64>),
    /// The widget picker has selected this widget.
    WidgetSelectedInInspector(WidgetId),
    /// A new [layer] should be created with the widget as root.
    ///
    /// The given [`Point`] must be in the window's coordinate space.
    ///
    /// [layer]: crate::doc::masonry_concepts#layers
    NewLayer(LayerType, NewWidget<dyn Widget>, Point),
    /// The layer with the given widget as root should be removed.
    RemoveLayer(WidgetId),
    /// The layer with the given widget as root should be repositioned to the specified point.
    ///
    /// The given [`Point`] must be in the window's coordinate space.
    RepositionLayer(WidgetId, Point),
}

/// State of the widget inspector. Useful for debugging.
///
/// Widget inspector is WIP. It should get its own standalone documentation.
pub(crate) struct InspectorState {
    pub(crate) is_picking_widget: bool,
    pub(crate) hovered_widget: Option<WidgetId>,
}

impl RenderRoot {
    /// Creates a new `RenderRoot` with the given options.
    ///
    /// The provided root widget will always stay the root widget.
    /// (It cannot be changed later for another widget, only its children can change.)
    /// In case no widget is focused the event pass will target text events
    /// at the child of the root widget if it only has one child.
    ///
    /// Note that this doesn't create a window or start an event loop.
    /// The `masonry` crate doesn't provide a way to do that:
    /// look for `masonry_winit::app::run` instead.
    pub fn new(
        root_widget: NewWidget<impl Widget + ?Sized>,
        signal_sink: impl FnMut(RenderRootSignal) + 'static,
        options: RenderRootOptions,
    ) -> Self {
        let RenderRootOptions {
            default_properties,
            use_system_fonts,
            size_policy,
            size,
            scale_factor,
            test_font,
        } = options;
        let debug_paint = std::env::var("MASONRY_DEBUG_PAINT").is_ok_and(|it| !it.is_empty());

        // LayerStack can't use Dimensions::AUTO because it'll resolve to the window size.
        // Instead we want to always measure LayerStack, so it can measure its base layer.
        let layer_stack = LayerStack::new(root_widget)
            .with_props(Dimensions::MAX)
            .to_pod();

        let mut root = Self {
            layer_stack,
            window_node_id: AccessCtx::next_node_id(),
            size_policy,
            size,
            last_mouse_pos: None,
            default_properties,
            global_state: RenderRootState {
                signal_sink: Box::new(signal_sink),
                focused_widget: None,
                focused_path: Vec::new(),
                next_focused_widget: None,
                focus_anchor: None,
                focus_fallback: None,
                window_focused: true,
                scroll_request_targets: Vec::new(),
                hovered_path: Vec::new(),
                active_path: Vec::new(),
                pointer_capture_target: None,
                cursor_icon: CursorIcon::Default,
                font_context: FontContext {
                    collection: Collection::new(CollectionOptions {
                        system_fonts: use_system_fonts,
                        ..Default::default()
                    }),
                    source_cache: SourceCache::default(),
                },
                text_layout_context: LayoutContext::new(),
                mutate_callbacks: Vec::new(),
                is_ime_active: false,
                last_sent_ime_area: INVALID_IME_AREA,
                scene_cache: HashMap::new(),
                widget_tags: HashMap::new(),
                needs_pointer_pass: false,
                trace: PassTracing::from_env(),
                inspector_state: InspectorState {
                    is_picking_widget: false,
                    hovered_widget: None,
                },
                access_tree_active: false,
                scale_factor,
                debug_paint,
            },
            widget_arena: WidgetArena {
                nodes: TreeArena::new(),
            },
        };

        if let Some(test_font_data) = test_font {
            // We don't use `register_fonts` here because that requests a global relayout.
            // However, because we are not yet fully initialised (we are before the below call
            // to `run_rewrite_passes`), `request_layout_all` will panic, as the root hasn't
            // been inserted into the arena yet.
            let families = root
                .global_state
                .font_context
                .collection
                .register_fonts(test_font_data, None);
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

    pub(crate) fn root_id(&self) -> WidgetId {
        self.layer_stack.id()
    }

    /// `WidgetId` of the given layer's root widget.
    pub(crate) fn layer_root_id(&self, layer_idx: usize) -> WidgetId {
        let node_ref = self
            .widget_arena
            .nodes
            .find(self.root_id())
            .expect("root widget not in widget tree");
        let widget = &*node_ref.item.widget;

        let stack = (widget as &dyn Any).downcast_ref::<LayerStack>().unwrap();
        stack.layer_id(layer_idx)
    }

    pub(crate) fn root_state(&self) -> &WidgetState {
        &self
            .widget_arena
            .nodes
            .roots()
            .into_item(self.root_id())
            .expect("root widget not in widget tree")
            .item
            .state
    }

    pub(crate) fn root_state_mut(&mut self) -> &mut WidgetState {
        &mut self
            .widget_arena
            .nodes
            .roots_mut()
            .into_item_mut(self.layer_stack.id())
            .expect("root widget not in widget tree")
            .item
            .state
    }

    // --- MARK: WINDOW_EVENT
    /// Handles a window event.
    pub fn handle_window_event(&mut self, event: WindowEvent) -> Handled {
        match event {
            WindowEvent::Rescale(scale_factor) => {
                self.global_state.scale_factor = scale_factor;
                self.request_render_all();
                Handled::Yes
            }
            WindowEvent::Resize(size) => {
                self.size = size;
                self.root_state_mut().request_layout = true;
                self.root_state_mut().set_needs_layout(true);
                self.run_rewrite_passes();
                Handled::Yes
            }
            WindowEvent::AnimFrame(duration) => {
                run_update_anim_pass(self, duration.as_nanos() as u64);
                self.run_rewrite_passes();

                Handled::Yes
            }
            WindowEvent::EnableAccessTree => {
                self.global_state.access_tree_active = true;
                // AccessKit expects the next update to have include
                // a description of every single node.
                self.request_access_all();
                self.global_state
                    .emit_signal(RenderRootSignal::RequestRedraw);
                Handled::Yes
            }
            WindowEvent::DisableAccessTree => {
                self.global_state.access_tree_active = false;
                Handled::Yes
            }
        }
    }

    // --- MARK: PUB FUNCTIONS
    /// Handles a pointer event.
    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> Handled {
        let _span = info_span!("pointer_event");
        let handled = run_on_pointer_event_pass(self, &event);
        run_update_pointer_pass(self);
        self.run_rewrite_passes();

        handled
    }

    /// Handles a text event.
    pub fn handle_text_event(&mut self, event: TextEvent) -> Handled {
        let _span = info_span!("text_event");
        let handled = run_on_text_event_pass(self, &event);
        run_update_focus_pass(self);

        if matches!(event, TextEvent::Ime(Ime::Enabled)) {
            // Reset the last sent IME area, as the platform reset the IME state and may have
            // forgotten it.
            self.global_state.last_sent_ime_area = INVALID_IME_AREA;
        }
        self.run_rewrite_passes();

        handled
    }

    /// Handles an accesskit event.
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
    pub fn register_fonts(&mut self, data: Blob<u8>) -> Vec<(FamilyId, Vec<FontInfo>)> {
        let ret = self
            .global_state
            .font_context
            .collection
            .register_fonts(data, None);
        run_update_fonts_pass(self);
        ret
    }

    /// Redraws the window.
    ///
    /// Returns an update to the accessibility tree and a Vello scene representing
    /// the widget tree's current state.
    pub fn redraw(&mut self) -> (Scene, Option<TreeUpdate>) {
        self.run_rewrite_passes();

        let access_tree_active = self.global_state.access_tree_active;

        // TODO - Handle invalidation regions
        let scene = run_paint_pass(self);
        let tree_update = access_tree_active
            .then(|| run_accessibility_pass(self, self.global_state.scale_factor));
        (scene, tree_update)
    }

    /// Returns the current icon that the mouse should display.
    pub fn cursor_icon(&self) -> CursorIcon {
        self.global_state.cursor_icon
    }

    // --- MARK: ACCESS WIDGETS
    /// Returns a [`WidgetRef`] to the root widget of the given [layer](crate::doc::masonry_concepts#layers).
    pub fn get_layer_root(&self, layer_idx: usize) -> WidgetRef<'_, dyn Widget> {
        self.get_widget(self.layer_root_id(layer_idx))
            .expect("layer root not in widget tree")
    }

    /// Returns a [`WidgetRef`] to a specific widget.
    pub fn get_widget(&self, id: WidgetId) -> Option<WidgetRef<'_, dyn Widget>> {
        let node_ref = self.widget_arena.nodes.find(id)?;

        let children = node_ref.children;
        let widget = &*node_ref.item.widget;
        let state = &node_ref.item.state;
        let properties = &node_ref.item.properties;

        let ctx = QueryCtx {
            global_state: &self.global_state,
            widget_state: state,
            properties: PropertiesRef {
                set: properties,
                default_map: self.default_properties.for_widget(widget.type_id()),
            },
            children,
            default_properties: &self.default_properties,
        };
        Some(WidgetRef { ctx, widget })
    }

    /// Returns a [`WidgetRef`] to the widget with the given tag.
    pub fn get_widget_with_tag<W: Widget + FromDynWidget + ?Sized>(
        &self,
        tag: WidgetTag<W>,
    ) -> Option<WidgetRef<'_, W>> {
        let id = self.global_state.widget_tags.get(&tag.inner)?;
        let widget_ref = self.get_widget(*id)?;
        let widget_ref = widget_ref.downcast().expect("wrong tag type");
        Some(widget_ref)
    }

    /// Checks if a widget with the given id is in the tree.
    pub fn has_widget(&self, id: WidgetId) -> bool {
        self.widget_arena.has(id)
    }

    /// Returns a [`WidgetMut`] to the root widget of the [base layer](crate::doc::masonry_concepts#layers).
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    pub fn edit_base_layer<R>(&mut self, f: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R) -> R {
        let layer_id = self.layer_root_id(0);
        let res = mutate_widget(self, layer_id, f);

        self.run_rewrite_passes();

        res
    }

    /// Returns a [`WidgetMut`] to the root widget of the given [layer](crate::doc::masonry_concepts#layers).
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds.
    pub fn edit_layer<R>(
        &mut self,
        layer_idx: usize,
        f: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R,
    ) -> R {
        let layer_id = self.layer_root_id(layer_idx);
        let res = mutate_widget(self, layer_id, f);

        self.run_rewrite_passes();

        res
    }

    /// Returns a [`WidgetMut`] to a specific widget.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    ///
    /// # Panics
    ///
    /// Panics if there is no widget with the given id in the tree.
    #[track_caller]
    pub fn edit_widget<R>(
        &mut self,
        id: WidgetId,
        f: impl FnOnce(WidgetMut<'_, dyn Widget>) -> R,
    ) -> R {
        if !self.widget_arena.has(id) {
            panic!("Could not find widget {id} in tree.");
        }

        let res = mutate_widget(self, id, f);

        self.run_rewrite_passes();

        res
    }

    /// Returns a [`WidgetMut`] to the widget with the given tag.
    ///
    /// Because of how `WidgetMut` works, it can only be passed to a user-provided callback.
    #[track_caller]
    pub fn edit_widget_with_tag<R, W: Widget + FromDynWidget + ?Sized>(
        &mut self,
        tag: WidgetTag<W>,
        f: impl FnOnce(WidgetMut<'_, W>) -> R,
    ) -> R {
        let Some(id) = self.global_state.widget_tags.get(&tag.inner).copied() else {
            panic!("Could not find widget with tag '{tag}' in widget tree.");
        };

        let res = mutate_widget(self, id, |mut widget_mut| f(widget_mut.downcast()));

        self.run_rewrite_passes();

        res
    }

    /// Adds a new layer at the end of the stack, with the given widget as its root, at the given position.
    ///
    /// The given `pos` must be in the window's coordinate space.
    pub fn add_layer(&mut self, root: NewWidget<impl Widget + ?Sized>, pos: Point) {
        debug!("added layer to stack");
        mutate_widget(self, self.root_id(), |mut layer_stack| {
            let mut layer_stack = layer_stack.downcast::<LayerStack>();
            LayerStack::add_layer(&mut layer_stack, root, pos);
        });

        self.run_rewrite_passes();
    }

    /// Removes the layer with the given widget as root.
    ///
    /// The base layer cannot be removed.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the the intended layer the base layer or the
    /// intended layer is not found.
    pub fn remove_layer(&mut self, root_id: WidgetId) {
        mutate_widget(self, self.root_id(), |mut layer_stack| {
            let mut layer_stack = layer_stack.downcast::<LayerStack>();
            LayerStack::remove_layer(&mut layer_stack, root_id);
        });

        self.run_rewrite_passes();
    }

    /// Repositions the layer with the given widget as root.
    ///
    /// The given `new_origin` must be in the window's coordinate space.
    ///
    /// The base layer cannot be repositioned.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the the intended layer the base layer or the
    /// intended layer is not found.
    pub fn reposition_layer(&mut self, root_id: WidgetId, new_origin: Point) {
        mutate_widget(self, self.root_id(), |mut layer_stack| {
            let mut layer_stack = layer_stack.downcast::<LayerStack>();
            LayerStack::reposition_layer(&mut layer_stack, root_id, new_origin);
        });

        self.run_rewrite_passes();
    }

    /// Returns the current size of the window.
    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub(crate) fn get_kurbo_size(&self) -> Size {
        let size = self.size.to_logical(self.global_state.scale_factor);
        Size::new(size.width, size.height)
    }

    // --- MARK: REWRITE PASSES
    /// Runs all rewrite passes on widget tree.
    ///
    /// Rewrite passes are passes which occur after external events, and
    /// update flags and internal values to a consistent state.
    ///
    /// See the [passes documentation](crate::doc::pass_system) for details.
    pub(crate) fn run_rewrite_passes(&mut self) {
        const REWRITE_PASSES_MAX: usize = 4;

        for _ in 0..REWRITE_PASSES_MAX {
            // Note: this code doesn't do any short-circuiting, because each pass is
            // expected to have its own early exits.
            // Calling a run_xxx_pass should always be very fast if the pass doesn't need to do anything.

            run_mutate_pass(self);
            run_update_widget_tree_pass(self);
            run_update_disabled_pass(self);
            run_update_stashed_pass(self);
            run_update_focusable_pass(self);
            run_update_focus_pass(self);
            run_layout_pass(self);
            run_update_scroll_pass(self);
            run_compose_pass(self);
            run_update_pointer_pass(self);

            if !self.needs_rewrite_passes() {
                break;
            }
        }

        if self.needs_rewrite_passes() {
            warn!(
                "All rewrite passes have run {REWRITE_PASSES_MAX} times, but invalidations are still set"
            );
            // To avoid an infinite loop, we delay re-running the passes until the next frame.
            self.global_state
                .emit_signal(RenderRootSignal::RequestRedraw);
        }

        if self.root_state().needs_anim {
            self.global_state
                .emit_signal(RenderRootSignal::RequestAnimFrame);
        }

        // We request a redraw if either the render tree or the accessibility
        // tree needs to be rebuilt. Usually both are rebuilt at the same time.
        // A redraw will trigger a rebuild of the accessibility tree.
        if self.root_state().needs_paint || self.needs_accessibility() {
            self.global_state
                .emit_signal(RenderRootSignal::RequestRedraw);
        }

        if self.global_state.is_ime_active {
            let widget = self
                .global_state
                .focused_widget
                .expect("IME is active without a focused widget");
            let ime_area = self.widget_arena.get_state(widget).get_ime_area();
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

    // TODO - Factor out into "visit_all" method?

    pub(crate) fn request_access_all(&mut self) {
        fn request_access_all_in(node: ArenaMut<'_, WidgetArenaNode>) {
            let children = node.children;
            let widget = &mut *node.item.widget;
            let state = &mut node.item.state;

            state.needs_accessibility = true;
            state.request_accessibility = true;

            let id = state.id;
            recurse_on_children(id, widget, children, |node| {
                request_access_all_in(node);
            });
        }

        let root_node = self.widget_arena.get_node_mut(self.root_id());
        request_access_all_in(root_node);
        self.global_state
            .emit_signal(RenderRootSignal::RequestRedraw);
    }

    pub(crate) fn request_render_all(&mut self) {
        fn request_render_all_in(node: ArenaMut<'_, WidgetArenaNode>) {
            let children = node.children;
            let widget = &mut *node.item.widget;
            let state = &mut node.item.state;

            state.needs_paint = true;
            state.needs_accessibility = true;
            state.request_pre_paint = true;
            state.request_paint = true;
            state.request_accessibility = true;
            state.request_post_paint = true;

            let id = state.id;
            recurse_on_children(id, widget, children, |node| {
                request_render_all_in(node);
            });
        }

        let root_node = self.widget_arena.get_node_mut(self.root_id());
        request_render_all_in(root_node);
        self.global_state
            .emit_signal(RenderRootSignal::RequestRedraw);
    }

    /// Checks whether the given id points to a widget that is "interactive".
    /// i.e. not disabled or stashed.
    /// Only interactive widgets can have text focus or pointer capture.
    pub(crate) fn is_still_interactive(&self, id: WidgetId) -> bool {
        let Some(node) = self.widget_arena.nodes.find(id) else {
            return false;
        };
        let state = &node.item.state;

        !state.is_stashed && !state.is_disabled
    }

    /// Returns the [`WidgetId`] of the [focused widget](crate::doc::masonry_concepts#text-focus).
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.global_state.focused_widget
    }

    /// Returns the [`WidgetId`] of the widget which [captures pointer events](crate::doc::masonry_concepts#pointer-capture).
    pub fn pointer_capture_target(&self) -> Option<WidgetId> {
        self.global_state.pointer_capture_target
    }

    /// Sets the [focused widget](crate::doc::masonry_concepts#text-focus)
    /// and the [focus anchor](crate::doc::masonry_concepts#focus-anchor).
    ///
    /// Returns false if the widget is not found in the tree or can't be focused.
    pub fn focus_on(&mut self, id: Option<WidgetId>) -> bool {
        if let Some(id) = id
            && !self.is_still_interactive(id)
        {
            return false;
        }
        self.global_state.next_focused_widget = id;
        self.global_state.focus_anchor = id;
        self.run_rewrite_passes();
        true
    }

    /// Sets the [focus fallback](crate::doc::masonry_concepts#focus-fallback).
    ///
    /// Returns false if the widget is not found in the tree or can't be focused.
    pub fn set_focus_fallback(&mut self, id: Option<WidgetId>) -> bool {
        if let Some(id) = id
            && !self.is_still_interactive(id)
        {
            return false;
        }
        self.global_state.focus_fallback = id;
        true
    }

    /// Returns true if the widget tree is waiting for an animation frame.
    pub fn needs_anim(&self) -> bool {
        self.root_state().needs_anim
    }

    /// Returns true if the accessibility tree needs to be rebuilt.
    ///
    /// This will be inhibited if `access_tree_active` is false.
    pub fn needs_accessibility(&self) -> bool {
        self.global_state.access_tree_active && self.root_state().needs_accessibility
    }

    /// Returns `true` if something requires a rewrite pass or a re-render.
    pub fn needs_rewrite_passes(&self) -> bool {
        self.root_state().needs_rewrite_passes() || self.global_state.needs_rewrite_passes()
    }

    // TODO - Remove?
    #[doc(hidden)]
    pub fn emit_signal(&mut self, signal: RenderRootSignal) {
        self.global_state.emit_signal(signal);
    }
}

impl RenderRootState {
    /// Sends a signal to the runner of this app, which allows global actions to be triggered by a widget.
    pub(crate) fn emit_signal(&mut self, signal: RenderRootSignal) {
        (self.signal_sink)(signal);
    }

    /// Does something in this state indicate that the rewrite passes need to be reran.
    ///
    /// This is checked in conjunction with [`WidgetState::needs_rewrite_passes`] - if
    /// either returns true, the fixed point loop of the rewrite passes will be run again.
    /// All passes have a fast-path exit check; these together are the union of those checks.
    pub(crate) fn needs_rewrite_passes(&self) -> bool {
        self.needs_pointer_pass
            || self.focused_widget != self.next_focused_widget
            || !self.mutate_callbacks.is_empty()
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
