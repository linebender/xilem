// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The transform for all views can be modified similar to CSS transforms.

use std::f64::consts::{PI, TAU};

use winit::error::EventLoopError;
use xilem::style::Style as _;
use xilem::view::{GridExt as _, button, grid, label, sized_box, transformed};
use xilem::{Affine, Color, EventLoop, Vec2, WidgetView, WindowOptions, Xilem};

struct TransformsGame {
    rotation: f64,
    translation: Vec2,
    scale: f64,
}

impl TransformsGame {
    fn view(&mut self) -> impl WidgetView<Self> + use<> {
        let rotation_correct = (self.rotation % TAU).abs() < 0.001;
        let scale_correct = self.scale >= 0.99 && self.scale <= 1.01;
        let translation_correct = self.translation.x == 0.0 && self.translation.y == 0.0;
        let everything_correct = rotation_correct && scale_correct && translation_correct;

        let status = if everything_correct {
            label("Great success!")
                .color(Color::new([0.0, 0.0, 1.0, 1.0]))
                .text_size(30.0)
        } else {
            let rotation_mark = if rotation_correct { "✓" } else { "⨯" };
            let scale_mark = if scale_correct { "✓" } else { "⨯" };
            let translation_mark = if translation_correct { "✓" } else { "⨯" };
            label(format!(
                "rotation: {rotation_mark}\nscale: {scale_mark}\ntranslation: {translation_mark}"
            ))
        };

        let bg_color = if everything_correct {
            [0.0, 1.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0, 0.2]
        };

        let status = sized_box(status).background_color(Color::new(bg_color));
        // Every view can be transformed similar as with CSS transforms in the web.
        // Currently only 2D transforms are supported.
        // Note that the order of the transformations is relevant.
        let transformed_status = transformed(
            // In an actual app, you wouldn't use both `transformed` and `.transform`.
            // This is here to validate that Xilem's support for nested `Transformed`
            // values works as expected.
            status.transform(Affine::translate(self.translation)),
        )
        .rotate(self.rotation)
        .scale(self.scale);

        let controls = (
            button("↶", |this: &mut Self| {
                this.rotation -= PI * 0.125;
            })
            .grid_pos(0, 0),
            button("↑", |this: &mut Self| {
                this.translation.y -= 10.0;
            })
            .grid_pos(1, 0),
            button("↷", |this: &mut Self| {
                this.rotation += PI * 0.125;
            })
            .grid_pos(2, 0),
            button("←", |this: &mut Self| {
                this.translation.x -= 10.0;
            })
            .grid_pos(0, 1),
            button("→", |this: &mut Self| {
                this.translation.x += 10.0;
            })
            .grid_pos(2, 1),
            button("-", |this: &mut Self| {
                // 2 ^ (1/3) for 3 clicks to reach the target.
                this.scale /= 1.2599210498948732;
            })
            .grid_pos(0, 2),
            button("↓", |this: &mut Self| {
                this.translation.y += 10.0;
            })
            .grid_pos(1, 2),
            button("+", |this: &mut Self| {
                this.scale *= 1.2599210498948732;
            })
            .grid_pos(2, 2),
        );

        grid((controls, transformed_status.grid_pos(1, 1)), 3, 3)
    }
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        TransformsGame {
            rotation: PI * 0.25,
            translation: Vec2::new(20.0, 30.0),
            scale: 2.0,
        },
        TransformsGame::view,
        WindowOptions::new("Transforms"),
    );
    app.run_in(EventLoop::builder())?;
    Ok(())
}
