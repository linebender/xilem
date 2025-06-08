// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{DefaultProperties, Widget};
use masonry::dpi::LogicalSize;
use masonry::theme::default_property_set;
use masonry_winit::app::{MasonryUserEvent, WindowId};
use winit::error::EventLoopError;
use winit::window::{Window, WindowAttributes};

use masonry::peniko::{Blob, Color};
use masonry_winit::app::EventLoopBuilder;

use crate::WidgetView;
use crate::driver::MasonryDriver;

/// Runtime builder.
#[must_use = "A Xilem app does nothing unless ran."]
pub struct Xilem<State, Logic> {
    state: State,
    logic: Logic,
    runtime: tokio::runtime::Runtime,
    default_properties: Option<DefaultProperties>,
    background_color: Color,
    // Font data to include in loading.
    fonts: Vec<Blob<u8>>,
}

impl<State, Logic, View> Xilem<State, Logic>
where
    Logic: FnMut(&mut State) -> View,
    View: WidgetView<State>,
{
    /// Initialize the builder state for your app.
    pub fn new(state: State, logic: Logic) -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        Self {
            state,
            logic,
            runtime,
            default_properties: None,
            background_color: Color::BLACK,
            fonts: Vec::new(),
        }
    }

    /// Load a font when this `Xilem` is run.
    ///
    /// This is an interim API whilst font lifecycles are determined.
    pub fn with_font(mut self, data: impl Into<Blob<u8>>) -> Self {
        self.fonts.push(data.into());
        self
    }

    /// Sets main window background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    // TODO: Find better ways to customize default property set.
    /// Sets default properties of widget tree.
    pub fn with_default_properties(mut self, default_properties: DefaultProperties) -> Self {
        self.default_properties = Some(default_properties);
        self
    }

    // TODO: Make windows a specific view
    /// Run app with default window attributes.
    pub fn run_windowed(
        self,
        // We pass in the event loop builder to allow
        // This might need to be generic over the event type?
        event_loop: EventLoopBuilder,
        window_title: String,
    ) -> Result<(), EventLoopError>
    where
        State: 'static,
        Logic: 'static,
        View: 'static,
    {
        let window_size = LogicalSize::new(600., 800.);
        let window_attributes = Window::default_attributes()
            .with_title(window_title)
            .with_resizable(true)
            .with_min_inner_size(window_size);
        self.run_windowed_in(event_loop, window_attributes)
    }

    // TODO: Make windows into a custom view
    /// Run app with custom window attributes.
    pub fn run_windowed_in(
        mut self,
        mut event_loop: EventLoopBuilder,
        window_attributes: WindowAttributes,
    ) -> Result<(), EventLoopError>
    where
        State: 'static,
        Logic: 'static,
        View: 'static,
    {
        let event_loop = event_loop.build()?;
        let proxy = event_loop.create_proxy();
        let default_properties = self
            .default_properties
            .take()
            .unwrap_or_else(default_property_set);
        let (driver, window_id, root_widget) =
            self.into_driver_and_window(move |event| proxy.send_event(event).map_err(|err| err.0));
        masonry_winit::app::run_with(
            event_loop,
            vec![(window_id, window_attributes, root_widget)],
            driver,
            default_properties,
        )
    }

    /// Builds the [`MasonryDriver`] and also returns the window id and the root widget.
    ///
    /// The given event sink function sends the given event to the event loop
    /// and returns the given event as an error in case the event loop is stopped.
    pub fn into_driver_and_window(
        self,
        event_sink: impl Fn(MasonryUserEvent) -> Result<(), MasonryUserEvent> + Send + Sync + 'static,
    ) -> (
        MasonryDriver<State, Logic, View, View::ViewState>,
        WindowId,
        Box<dyn Widget>,
    ) {
        MasonryDriver::new(self.state, self.logic, event_sink, self.runtime, self.fonts)
    }
}
