// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Playground for the `Split` widget.
//!
//! Try:
//! - Dragging the divider.
//! - Clicking the divider, then using arrow keys (Shift = bigger step).
//! - Home/End to jump to the min/max allowed positions.

// On Windows platform, don't show a console when opening the app.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use masonry::core::{ErasedAction, NewWidget, Widget as _, WidgetId, WidgetTag};
use masonry::dpi::LogicalSize;
use masonry::layout::{AsUnit, Length};
use masonry::theme::default_property_set;
use masonry::widgets::{Button, ButtonPress, Flex, Label, Split, SplitPoint};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

#[derive(Clone, Copy, Debug, PartialEq)]
enum SplitPointMode {
    Fraction,
    FromStart,
    FromEnd,
}

const SPLIT_TAG: WidgetTag<Split<Flex, Flex>> = WidgetTag::named("split");
const STATUS_TAG: WidgetTag<Label> = WidgetTag::named("status");
const FRACTION_BTN: WidgetTag<Button> = WidgetTag::named("fraction-btn");
const START_BTN: WidgetTag<Button> = WidgetTag::named("start-btn");
const END_BTN: WidgetTag<Button> = WidgetTag::named("end-btn");

struct Driver {
    window_id: WindowId,
    mode: SplitPointMode,
}

impl Driver {
    fn set_mode(&mut self, ctx: &mut DriverCtx<'_, '_>, mode: SplitPointMode) {
        self.mode = mode;

        let render_root = ctx.render_root(self.window_id);
        render_root.edit_widget_with_tag(SPLIT_TAG, |mut split| {
            let split_point = match mode {
                SplitPointMode::Fraction => SplitPoint::Fraction(0.5),
                SplitPointMode::FromStart => SplitPoint::FromStart(220.px()),
                SplitPointMode::FromEnd => SplitPoint::FromEnd(220.px()),
            };
            Split::set_split_point(&mut split, split_point);
        });

        render_root.edit_widget_with_tag(STATUS_TAG, |mut label| {
            let text = match mode {
                SplitPointMode::Fraction => "Mode: fraction (0.5)",
                SplitPointMode::FromStart => "Mode: from start (220px)",
                SplitPointMode::FromEnd => "Mode: from end (220px)",
            };
            Label::set_text(&mut label, text);
        });
    }
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown window");
        if !action.is::<ButtonPress>() {
            return;
        }

        let render_root = ctx.render_root(window_id);
        let fraction_id = render_root.get_widget_with_tag(FRACTION_BTN).unwrap().id();
        let start_id = render_root.get_widget_with_tag(START_BTN).unwrap().id();
        let end_id = render_root.get_widget_with_tag(END_BTN).unwrap().id();

        if widget_id == fraction_id {
            self.set_mode(ctx, SplitPointMode::Fraction);
        } else if widget_id == start_id {
            self.set_mode(ctx, SplitPointMode::FromStart);
        } else if widget_id == end_id {
            self.set_mode(ctx, SplitPointMode::FromEnd);
        }
    }
}

fn make_widget_tree() -> NewWidget<impl masonry::core::Widget> {
    const SPACING: Length = Length::const_px(8.0);

    let controls = Flex::row()
        .with_fixed(NewWidget::new_with_tag(
            Button::with_text("Fraction"),
            FRACTION_BTN,
        ))
        .with_fixed_spacer(SPACING)
        .with_fixed(NewWidget::new_with_tag(
            Button::with_text("From start"),
            START_BTN,
        ))
        .with_fixed_spacer(SPACING)
        .with_fixed(NewWidget::new_with_tag(
            Button::with_text("From end"),
            END_BTN,
        ));

    let status = NewWidget::new_with_tag(
        Label::new(
            "Click the divider, then use arrow keys (Shift = bigger step), Home/End to jump.",
        ),
        STATUS_TAG,
    );

    let left = Flex::column()
        .with_fixed(
            Label::new("Left pane")
                .with_style(masonry::core::StyleProperty::FontSize(18.0))
                .with_auto_id(),
        )
        .with_fixed(Label::new("Drag / keyboard-resize the divider.").with_auto_id());

    let right = Flex::column()
        .with_fixed(
            Label::new("Right pane")
                .with_style(masonry::core::StyleProperty::FontSize(18.0))
                .with_auto_id(),
        )
        .with_fixed(Label::new("Try switching split-point modes.").with_auto_id());

    let split = NewWidget::new_with_tag(
        Split::new(NewWidget::new(left), NewWidget::new(right))
            .min_lengths(80.px(), 80.px())
            .min_bar_area(12.px()),
        SPLIT_TAG,
    );

    NewWidget::new(
        Flex::column()
            .with_fixed(controls.with_auto_id())
            .with_fixed_spacer(SPACING)
            .with_fixed(status)
            .with_fixed_spacer(SPACING)
            .with(split, 1.0),
    )
}

fn main() {
    let window_size = LogicalSize::new(900.0, 600.0);
    let window_attributes = Window::default_attributes()
        .with_title("Split playground")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    let driver = Driver {
        window_id: WindowId::next(),
        mode: SplitPointMode::Fraction,
    };

    masonry_winit::app::run(
        vec![NewWindow::new_with_id(
            driver.window_id,
            window_attributes,
            make_widget_tree().erased(),
        )],
        driver,
        default_property_set(),
    )
    .unwrap();
}
