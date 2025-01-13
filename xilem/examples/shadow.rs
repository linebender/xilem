//! A demo showing how to use shadows in Xilem.
//!
//! This example demonstrates:
//! - How to create and customize shadow effects
//! - How to build responsive layouts that work on both desktop and mobile
//! - How to structure complex widget hierarchies
//! - Component composition patterns for reusability

use masonry::widget::{CrossAxisAlignment, MainAxisAlignment};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::window::Window;
use xilem::palette::css;
use xilem::view::{
    button, flex, label, portal, prose, sized_box, slider, Axis, FlexExt, FlexSpacer, Padding,
};
use xilem::{Color, EventLoop, EventLoopBuilder, TextAlignment, WidgetView, Xilem};

/// The main application state containing all shadow configuration parameters
struct AppState {
    /// Horizontal offset of the shadow
    offset_x: f64,
    /// Vertical offset of the shadow
    offset_y: f64,
    /// Blur radius of the shadow
    blur_radius: f64,
    /// Spread radius of the shadow
    spread_radius: f64,
    /// Corner radius of the shadow
    corner_radius: f64,
    /// Color of the shadow
    shadow_color: Color,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            offset_x: 12.5,
            offset_y: 12.5,
            blur_radius: 20.0,
            spread_radius: 3.0,
            corner_radius: 24.0,
            shadow_color: css::RED,
        }
    }
}

/// A reusable control component for adjusting shadow parameters
struct ShadowControl {
    /// Label text for the control
    label: String,
    /// Current value of the control
    value: f64,
    /// Valid range for the control value
    range: std::ops::Range<f64>,
}

impl ShadowControl {
    /// Creates a new shadow control with the given label, initial value and range
    fn new(label: &str, value: f64, range: std::ops::Range<f64>) -> Self {
        Self {
            label: label.to_string(),
            value,
            range,
        }
    }

    /// Renders the control as a labeled slider
    fn view<F>(&self, on_change: F) -> impl WidgetView<AppState>
    where
        F: Fn(&mut AppState, f64) + 'static + Send + Sync,
    {
        flex((
            label(format!("{}: {:.1}", self.label, self.value)),
            slider(self.range.clone(), self.value, on_change)
                .with_hover_glow_color(css::LIGHT_BLUE)
                .with_hover_glow_blur_radius(8.0)
                .with_hover_glow_spread_radius(2.0)
                .with_step(0.1),
        ))
        .direction(Axis::Vertical)
        .cross_axis_alignment(CrossAxisAlignment::Center)
    }
}

impl AppState {
    /// Renders the control panel containing all shadow adjustment controls
    fn controls_panel(&mut self) -> impl WidgetView<Self> {
        sized_box(
            flex((
                prose("Shadow Controls")
                    .text_size(20.)
                    .alignment(TextAlignment::Middle),
                ShadowControl::new("Offset X", self.offset_x, -50.0..50.0)
                    .view(|state, val| state.offset_x = val),
                ShadowControl::new("Offset Y", self.offset_y, -50.0..50.0)
                    .view(|state, val| state.offset_y = val),
                ShadowControl::new("Blur Radius", self.blur_radius, 0.0..50.0)
                    .view(|state, val| state.blur_radius = val),
                ShadowControl::new("Spread Radius", self.spread_radius, -20.0..20.0)
                    .view(|state, val| state.spread_radius = val),
                ShadowControl::new("Corner Radius", self.corner_radius, 0.0..150.0)
                    .view(|state, val| state.corner_radius = val),
                sized_box(
                    flex((
                        FlexSpacer::Flex(10.0),
                        button("Toggle Color", |state: &mut AppState| {
                            state.shadow_color = if state.shadow_color == css::BLUE {
                                css::RED
                            } else {
                                css::BLUE
                            };
                        }),
                        FlexSpacer::Flex(10.0),
                    ))
                    .direction(Axis::Horizontal),
                )
                .padding(8.) // 添加内边距
                .rounded(4.),
            ))
            .direction(Axis::Vertical)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_alignment(MainAxisAlignment::Start),
        )
        .padding(16.)
    }

    /// Renders the preview panel showing the shadow effect
    fn preview_panel(&self) -> impl WidgetView<Self> {
        sized_box(
            flex(
                label("label")
                    .text_size(50.0)
                    .brush(css::BLACK)
                    .alignment(TextAlignment::Middle),
            )
            .direction(Axis::Vertical)
            .main_axis_alignment(MainAxisAlignment::Center),
        )
        .width(200.0)
        .height(200.0)
        .background(Color::WHITE)
        .rounded(self.corner_radius)
        .shadow(
            self.shadow_color,
            (self.offset_x, self.offset_y),
            self.blur_radius,
            self.spread_radius,
            Some(self.corner_radius),
        )
    }

    /// Renders the main application view
    fn view(&mut self) -> impl WidgetView<Self> {
        flex((
            FlexSpacer::Fixed(40.),
            portal(
                flex((
                    sized_box(self.preview_panel())
                        .padding(Padding::all(16.))
                        .background(css::LIGHT_GRAY.with_alpha(0.1)),
                    self.controls_panel(),
                ))
                .direction(Axis::Vertical),
            )
            .flex(1.),
        ))
        .direction(Axis::Vertical)
        .must_fill_major_axis(true)
    }
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = AppState::default();
    let app = Xilem::new(data, AppState::view);

    let window_attributes = Window::default_attributes()
        .with_title("Shadow Example")
        .with_resizable(false) // 禁止调整窗口大小
        .with_inner_size(LogicalSize::new(400., 800.)); // 设置合适的竖屏尺寸

    app.run_windowed_in(event_loop, window_attributes)
}

#[cfg(not(target_os = "android"))]
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
