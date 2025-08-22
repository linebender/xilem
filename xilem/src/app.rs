// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::peniko::Blob;
use masonry_winit::app::{EventLoopBuilder, NewWindow, WindowId};

use std::iter::Once;
use std::sync::Arc;

use masonry::core::DefaultProperties;
use masonry::theme::default_property_set;
use masonry_winit::app::MasonryUserEvent;
use winit::error::EventLoopError;
use xilem_core::map_state;

use crate::window_options::WindowCallbacks;
use crate::{AnyWidgetView, MasonryDriver, WidgetView, WindowOptions};

/// Runtime builder.
#[must_use = "A Xilem app does nothing unless ran."]
pub struct Xilem<State, Logic> {
    state: State,
    logic: Logic,
    runtime: tokio::runtime::Runtime,
    default_properties: Option<DefaultProperties>,
    // Font data to include in loading.
    fonts: Vec<Blob<u8>>,
}

/// State type used by [`Xilem::new_simple`].
pub struct ExitOnClose<S> {
    state: S,
    running: bool,
}

impl<S> AppState for ExitOnClose<S> {
    fn keep_running(&self) -> bool {
        self.running
    }
}

type WindowTuple<State> = (WindowId, WindowOptions<State>, Box<AnyWidgetView<State>>);

impl<State>
    Xilem<
        ExitOnClose<State>,
        Box<dyn FnMut(&mut ExitOnClose<State>) -> Once<WindowTuple<ExitOnClose<State>>>>,
    >
{
    /// Create an app builder for a single window app with fixed window attributes
    /// that exits once the window is closed.
    ///
    /// If you want to have multiple windows or change e.g. the window title depending
    /// on the state you should instead use [`Xilem::new`] (which this function wraps).
    pub fn new_simple<View>(
        state: State,
        mut logic: impl FnMut(&mut State) -> View + 'static,
        window_options: WindowOptions<State>,
    ) -> Self
    where
        View: WidgetView<State>,
        State: 'static,
    {
        let window_id = WindowId::next();
        let callbacks = Arc::new(window_options.callbacks);
        Xilem::new_inner(
            ExitOnClose {
                state,
                running: true,
            },
            Box::new(move |ExitOnClose { state, .. }| {
                let callbacks = callbacks.clone();
                let on_close = move |wrapper: &mut ExitOnClose<_>| {
                    wrapper.running = false;
                    if let Some(on_close) = &callbacks.on_close {
                        on_close(&mut wrapper.state);
                    }
                };
                std::iter::once((
                    window_id,
                    WindowOptions {
                        reactive: window_options.reactive.clone(),
                        initial: window_options.initial.clone(),
                        callbacks: WindowCallbacks {
                            on_close: Some(Box::new(on_close)),
                        },
                    },
                    map_state(logic(state), |wrapper: &mut ExitOnClose<_>| {
                        &mut wrapper.state
                    })
                    .boxed(),
                ))
            }),
        )
    }
}

/// The trait [`Xilem::new`] expects to be implemented for the state.
///
/// [`Xilem::new_simple`] does not use this trait implementation.
pub trait AppState {
    /// Returns whether the application should keep running or exit.
    ///
    /// Is currently only checked after a close request.
    // TODO: check this after every state mutation
    fn keep_running(&self) -> bool;
}

impl<State, Logic, WindowIter> Xilem<State, Logic>
where
    State: AppState + 'static,
    Logic: FnMut(&mut State) -> WindowIter + 'static,
    WindowIter: Iterator<Item = (WindowId, WindowOptions<State>, Box<AnyWidgetView<State>>)>,
{
    /// Initialize the builder state for your app with an app logic function that returns a window iterator.
    pub fn new(state: State, logic: Logic) -> Self
    where
        State: AppState,
    {
        Self::new_inner(state, logic)
    }

    fn new_inner(state: State, logic: Logic) -> Self {
        Self {
            state,
            logic,
            runtime: tokio::runtime::Runtime::new().unwrap(),
            default_properties: None,
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

    // TODO: Find better ways to customize default property set.
    /// Sets default properties of widget tree.
    pub fn with_default_properties(mut self, default_properties: DefaultProperties) -> Self {
        self.default_properties = Some(default_properties);
        self
    }

    /// Run app with custom window attributes.
    pub fn run_in(mut self, mut event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
        let event_loop = event_loop.build()?;

        let proxy = event_loop.create_proxy();

        let (event_sender, event_receiver) = std::sync::mpsc::channel::<MasonryUserEvent>();

        let default_properties = self
            .default_properties
            .take()
            .unwrap_or_else(default_property_set);

        let sender = event_sender.clone();
        let (driver, windows) = self.into_driver_and_windows(move |event| {
            sender.send(event).map_err(|err| err.0)?;
            proxy.wake_up();

            Ok(())
        });

        masonry_winit::app::run_with(
            event_loop,
            event_sender,
            event_receiver,
            windows,
            driver,
            default_properties,
        )
    }

    /// Builds the [`MasonryDriver`] and the initial windows.
    ///
    /// The given event sink function sends the given event to the event loop
    /// and returns the given event as an error in case the event loop is stopped.
    pub fn into_driver_and_windows(
        self,
        proxy: impl Fn(MasonryUserEvent) -> Result<(), MasonryUserEvent> + Send + Sync + 'static,
    ) -> (MasonryDriver<State, Logic>, Vec<NewWindow>) {
        MasonryDriver::new(self.state, self.logic, proxy, self.runtime, self.fonts)
    }
}
