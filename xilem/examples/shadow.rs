 // Copyright 2024 the Xilem Authors
 // SPDX-License-Identifier: Apache-2.0

 //! Demonstrates how to use shadows in Xilem

 use masonry::widget::CrossAxisAlignment;
 use winit::error::EventLoopError;
 use xilem::palette::css;
use xilem::view::{button, flex, label, slider, sized_box, Axis, FlexExt as _,};
 use xilem::{Color, EventLoop, WidgetView, Xilem};


 struct AppState {
     offset_x: f64,
     offset_y: f64,
     blur_radius: f64,
     spread_radius: f64,
     corner_radius: f64,
     shadow_color: Color,
 }

impl Default for AppState {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur_radius: 0.0,
            spread_radius: 0.0,
            corner_radius: 0.0,
            shadow_color: Color::BLACK,
        }
    }
}
 fn app_logic(data: &mut AppState) -> impl WidgetView<AppState> {
     flex((
         // Controls column
         sized_box(
            flex((
                label("Shadow Controls").text_size(24.),
                label(format!("Offset X: {:.1}", data.offset_x)),
                slider(-50.0..50.0,data.offset_x, |state:&mut AppState, val| state.offset_x = val),
                label(format!("Offset Y: {:.1}", data.offset_y)),
                slider(-50.0..50.0,data.offset_y,  |state:&mut AppState, val| state.offset_y = val),
                label(format!("Blur Radius: {:.1}", data.blur_radius)),
                slider(0.0..50.0,data.blur_radius,  |state:&mut AppState, val| state.blur_radius = val),
                label(format!("Spread Radius: {:.1}", data.spread_radius)),
                slider(-20.0..20.0,data.spread_radius,  |state:&mut AppState, val| state.spread_radius = val),
                label(format!("Corner Radius: {:.1}", data.corner_radius)),
                slider(0.0..50.0,data.corner_radius,  |state:&mut AppState, val| state.corner_radius = val),
                button("Toggle Color", |state:&mut AppState| {
                    state.shadow_color = if state.shadow_color == Color::BLACK {
                        css::RED
                    } else {
                        Color::BLACK
                    };
                }),
            ))
            .direction(Axis::Vertical)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_alignment(masonry::widget::MainAxisAlignment::Start)
         )
         .padding(20.0)
         .flex(1.0),

         // Preview column
         sized_box(label("label"))
             .width(300.0)
             .height(300.0)
             .background(Color::WHITE)
             .rounded(data.corner_radius)
             .shadow(
                 data.shadow_color,
                 (data.offset_x, data.offset_y),
                 data.blur_radius,
                 data.spread_radius,
                 Some(data.corner_radius),
             )
             .flex(1.0),
     ))
     .direction(Axis::Horizontal)
     .cross_axis_alignment(CrossAxisAlignment::Center)
     .main_axis_alignment(masonry::widget::MainAxisAlignment::Center)
 }


 fn main() -> Result<(), EventLoopError> {
     let app = Xilem::new(AppState::default(), app_logic);
     app.run_windowed(EventLoop::with_user_event(), "Shadow Example".into())?;
     Ok(())
 }