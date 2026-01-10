// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A vello Scene can be used directly in Xilem.

use std::sync::atomic::{AtomicU32, Ordering};

use xilem::core::Edit;
use xilem::vello::Scene;
use xilem::vello::kurbo::{Affine, Circle, Size};
use xilem::vello::peniko::{Color, Fill};
use xilem::view::{canvas, text_button, zstack};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

/// Simple but suboptimal pseudo-random function, you shouldn't use this in production
fn rand_f32() -> f32 {
    static SEED: AtomicU32 = AtomicU32::new(123);
    SEED.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
        Some(x.wrapping_mul(1664525).wrapping_add(1013904223))
    })
    .unwrap() as f32
        * (1.0 / u32::MAX as f32)
}

#[derive(Default)]
struct Circles {
    circles: Vec<(Circle, Color)>,
    current_canvas_size: Size,
}

impl Circles {
    fn push_random_circle(&mut self) {
        let position = (
            rand_f32() as f64 * self.current_canvas_size.width,
            rand_f32() as f64 * self.current_canvas_size.height,
        );
        let radius = rand_f32() as f64 * self.current_canvas_size.height / 2.0;
        let color = Color::new([rand_f32(), rand_f32(), rand_f32(), 1.0]);
        self.circles.push((Circle::new(position, radius), color));
    }
    fn view(&mut self) -> impl WidgetView<Edit<Self>> + use<> {
        zstack((
            canvas(|state: &mut Self, _ctx, scene: &mut Scene, size: Size| {
                for (circle, color) in &state.circles {
                    scene.fill(Fill::NonZero, Affine::IDENTITY, *color, None, &circle);
                }
                state.current_canvas_size = size;
            }),
            text_button("Add Circle", Self::push_random_circle),
        ))
    }
}

fn main() -> Result<(), EventLoopError> {
    Xilem::new_simple(
        Circles::default(),
        Circles::view,
        WindowOptions::new("Canvas - Circles"),
    )
    .run_in(EventLoop::with_user_event())
}
