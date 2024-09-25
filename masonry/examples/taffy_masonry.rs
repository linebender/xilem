use dpi::LogicalSize;
use parley::layout::Alignment;
use winit::window::Window;
use masonry::app_driver::{AppDriver, DriverCtx};
use masonry::{Action, Color, WidgetId};
use masonry::widget::{Taffy, TaffyParams, Prose, RootWidget, SizedBox};


struct Driver {
}

impl AppDriver for Driver {
    fn on_action(&mut self, ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, action: Action) {

    }
}

pub fn main() {
    let label1 = SizedBox::new(
        Prose::new("Label 1")
            .with_text_size(14.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(150, 60, 90), 20.0);
    let label2 = SizedBox::new(
        Prose::new("Label 2")
            .with_text_size(10.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(40, 40, 80), 10.0);
    let label3 = SizedBox::new(
        Prose::new("Label 3: This is a long one. It will take up more space.")
            .with_text_size(10.0)
            .with_text_alignment(Alignment::Middle),
    )
        .border(Color::rgb8(20, 230, 80), 2.0);

    let driver = Driver {};

    // Arrange widgets in a 4 by 4 grid.
    let main_widget = Taffy::new(taffy::Style::default())
        .with_child(label1, TaffyParams::new())
        .with_child(label2, TaffyParams::new())
        .with_child(label3, TaffyParams::new());

    let window_size = LogicalSize::new(800.0, 500.0);
    let window_attributes = Window::default_attributes()
        .with_title("Taffy Layout")
        .with_resizable(true)
        .with_inner_size(window_size);

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        window_attributes,
        RootWidget::new(main_widget),
        driver,
    )
        .unwrap();
}
