use xilem::{view::View, App, AppLauncher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AppState {
    count: u32,
}

impl AppState {
    fn new() -> Self {
        Self { count: 1 }
    }
}

#[cfg(not(feature = "taffy"))]
fn app_logic(_data: &mut AppState) -> impl View<AppState> {
    "Error: this example requires the 'taffy' feature to be enabled"
}

#[cfg(feature = "taffy")]
fn app_logic(state: &mut AppState) -> impl View<AppState> {
    use taffy::style::{AlignItems, FlexWrap, JustifyContent};
    use taffy::style_helpers::length;
    use vello::peniko::Color;
    use xilem::view::{button, div, flex_column, flex_row, scroll_view};

    const COLORS: [Color; 4] = [
        Color::LIGHT_GREEN,
        Color::BLACK,
        Color::AZURE,
        Color::HOT_PINK,
    ];

    // Some logic, deriving state for the view from our app state
    let label = if state.count == 1 {
        "Square count: 1".to_string()
    } else {
        format!("Square count: {}", state.count)
    };

    // The actual UI Code starts here
    flex_column((

        // Header
        div(String::from("Xilem Example"))
            .with_background_color(Color::RED)
            .with_style(|s| s.padding = length(20.0)),

        scroll_view(

            // Body 
            flex_column((

                // Counter control buttons
                flex_row((
                    label,
                    button("increase", |state: &mut AppState| {
                        println!("clicked increase");
                        state.count += 1;
                    }),
                    button("decrease", |state: &mut AppState| {
                        println!("clicked decrease");
                        if state.count > 0 {
                            state.count -= 1;
                        }
                    }),
                    button("reset", |state: &mut AppState| {
                        println!("clicked reset");
                        state.count = 1;
                    }),
                ))
                .with_background_color(Color::BLUE_VIOLET)
                .with_style(|s| {
                    s.gap.width = length(20.0);
                    s.padding = length(20.0);
                    s.justify_content = Some(JustifyContent::Start);
                    s.align_items = Some(AlignItems::Center);
                }),

                // Description text
                div(String::from("The number of squares below is controlled by the counter above.\n\nTry clicking \"increase\" until the square count increases enough that the view becomes scrollable."))
                    .with_background_color(Color::RED)
                    .with_style(|s| s.padding = length(20.0)),

                // Lorem Ipsum text
                div(String::from("Lorem ipsum dolor sit amet, 汁投代経夫間費 consectetur adipiscing elit, sed უბიყუე do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."))
                    .with_background_color(Color::RED)
                    .with_style(|s| s.padding = length(20.0)),

                // Wrapping container (number of children controlled by counter)
                flex_row(
                    (0..state.count).map(|i| {
                        div(())
                            .with_background_color(COLORS[(i % 4) as usize])
                            .with_style(|s| {
                                s.size.width = length(200.0);
                                s.size.height = length(200.0);
                            })
                    }).collect::<Vec<_>>()
                )
                    .with_background_color(Color::FOREST_GREEN)
                    .with_style(|s| {
                        s.flex_grow = 1.0;
                        s.flex_wrap = FlexWrap::Wrap;
                        s.gap = length(20.0);
                        s.padding = length(20.0);
                    }),

            ))
            .with_style(|s| {
                s.gap.height = length(20.0);
                s.padding.left = length(20.0);
                s.padding.right = length(20.0);
                s.padding.top = length(20.0);
                s.padding.bottom = length(20.0);
            })
            .with_background_color(Color::WHITE)
        )
    )).with_style(|s| {
        s.padding.left = length(20.0);
        s.padding.right = length(20.0);
        s.padding.top = length(20.0);
        s.padding.bottom = length(20.0);
    })
    .with_background_color(Color::DARK_GRAY)
}

fn main() {
    let app = App::new(AppState::new(), app_logic);
    AppLauncher::new(app).run();
}
