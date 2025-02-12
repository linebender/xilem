// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An illustration of the various options for Xilem's Button/Button9 View
//! (based on Masonry Button/Button9 Widgets)
// TODOs:
// add rust code generating each element in a tooltip
// add the same code in a context menu as a "copy" command
// add URL support for doc links
// add non-desktop platforms
#![allow(
    unused_assignments,
    reason = "allows NOT having to track section increments {i+=1;}, removing the last one."
)]

use masonry::dpi::LogicalSize;
use winit::window::Window;
#[derive(Default)]
struct AppState {}

use masonry::core::ArcStr;
use masonry::peniko::Color;
use winit::error::EventLoopError;
use xilem::view::{
    button9_pad, flex, label, portal, prose, Axis, Prose,
};
use xilem::{palette::css, EventLoop, FontWeight, TextAlignment, WidgetView, Xilem};

fn title_prose(text: impl Into<ArcStr>) -> Prose {
    prose(text)
        .text_size(18.0)
        .alignment(TextAlignment::Justified)
        .brush(css::GREEN)
}
fn txt_prose(text: impl Into<ArcStr>) -> Prose {
    prose(text)
        .text_size(14.0)
        .alignment(TextAlignment::Start)
        .brush(Color::from_rgb8(0x11, 0x11, 0x11))
}

use masonry::kurbo::Insets;
const OFF0: Insets = Insets::uniform_xy(0., 0.);
// const OFF8: Insets = Insets::uniform_xy(8., 2.);
fn app_logic(_d: &mut AppState) -> impl WidgetView<AppState> {
    let _cb = |_d: &mut AppState| {}; // empty callback for empty buttons
                                      // let m_c = Color::from_rgb8(0x11, 0x11, 0x11); //main text
                                      // let l_c = LABEL_COLOR;
    let mut i = 1;

    portal(
    flex((
    (txt_prose("Xilem view::Button/Button9 formats vGit@25-02 (in a ↕-scrollable area)").text_size(18.0),
    if cfg!(debug_assertions) {txt_prose(
     "This is a debug build, so you can use github.com/linebender/xilem/tree/main/masonry#debugging-features:
        • F11 to toggle a rudimentary widget inspector
        • F12 to toggle paint widget layout rectangles")
    } else {txt_prose("This is not a debug build, so F11 widget inspector and F12 widget rectangles tools are not available)\ngithub.com/linebender/xilem/tree/main/masonry#debugging-features")},
    ),
    (title_prose(format!("§{i} button TBD")),),
    (title_prose(format!("§{i}a button9")),{i+=1;},
    txt_prose("  A button indicating 2 keys that activate it: ⏎Enter and an 'accelerator' I, but instead of tiny i̲ with a flexible position you have a permanent position with proper size and color highlight"),
    button9_pad(label("Confi̲rm").alignment(TextAlignment::Start).text_size(18.).weight(FontWeight::BOLD),Some(OFF0),|_|{println!("Confirm button pressed!");})
        .add3(label("⏎"        ).alignment(TextAlignment::Start).text_size(12.).brush(css::ORANGE).font("Cambria"),Some(Insets::new(4.,4.,2.,2.)),)
        .add4(label("I̲"        ).alignment(TextAlignment::Start).text_size(14.).brush(css::ORANGE).weight(FontWeight::BOLD),Some(Insets::new(4.,2.,4.,4.)),)
        ,
    ),
    (title_prose(format!("§{i}b button9 pad (Insets)")),
    txt_prose("  up to 9 labels added via `.add1–add9` methods, each with a per-label offset (with each of the 4 sides can be set separately)"),
    txt_prose("  ↓offsets are all 0 unless explicitly specified in the label with arrows indicating direction to the offset"),
    button9_pad(label("←5"  ).alignment(TextAlignment::Start),Some(Insets::new(5.,0.,0.,0.)),|_|{println!("Button1");})
        .add4(label("←1¦4→"	),Some(Insets::new(1.,0.,4.,0.)),)
        .add6(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add1(label("←1¦1→"	),Some(Insets::new(1.,0.,1.,0.)),)
        .add2(label("←2"   	).brush(css::ORANGE),Some(Insets::new(2.,0.,0.,0.)),)
        .add3(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add7(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add8(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add9(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        ,
    button9_pad(label("←5"  ).alignment(TextAlignment::Start),Some(Insets::new(5.,0.,0.,0.)),|_|{println!("Button2");})
        .add4(label("←1¦4→"	),Some(Insets::new(1.,0.,4.,0.)),)
        .add6(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add1(label("←1¦1→"	),Some(Insets::new(1.,0.,1.,0.)),)
        .add2(label("←5"   	).brush(css::ORANGE),Some(Insets::new(5.,0.,0.,0.)),)
        .add3(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add7(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add8(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add9(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        ,
    txt_prose("  ↑button width unchanged despite an increase in padding between ↖L1 and ↑L2 in the Top row (from max(1,2)=2 to max(1,5)=5) since it's still not bigger than padding between ←L4 and •L5 in the Middle row (max(4,5)=5), and the button width equals the max width of all rows to fit them all"),

    button9_pad(label("←5"  ).alignment(TextAlignment::Start),Some(Insets::new(5.,0.,0.,0.)),|_|{println!("Button1");})
        .add4(label("←1¦4→"	),Some(Insets::new(1.,0.,4.,0.)),)
        .add6(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add1(label("←1¦1→"	).brush(css::ORANGE),Some(Insets::new(1.,0.,1.,0.)),)
        .add2(label("←2"   	).brush(css::MEDIUM_SEA_GREEN),Some(Insets::new(2.,0.,0.,0.)),)
        .add3(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add7(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add8(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add9(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        ,
    button9_pad(label("←5"  ).alignment(TextAlignment::Start),Some(Insets::new(5.,0.,0.,0.)),|_|{println!("Button2");})
        .add4(label("←1¦4→"	),Some(Insets::new(1.,0.,4.,0.)),)
        .add6(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add1(label("←8¦1→"	).brush(css::ORANGE),Some(Insets::new(8.,0.,1.,0.)),)
        .add2(label("←5"   	).brush(css::MEDIUM_SEA_GREEN),Some(Insets::new(5.,0.,0.,0.)),)
        .add3(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add7(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add8(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        .add9(label(""     	),Some(Insets::new(0.,0.,0.,0.)),)
        ,
    txt_prose("  but moving ↖L1 to the right by 7 by increasing its left padding from 1 to 8 in addition to the increase in padding between ↖L1 and ↑L2 by 3 (from max(1,2)=2 to max(1,5)=5) makes the overall width of the Top row (+20 = +10⋅2 on both the left and the right sides to maintain symmetry vs. the central label) bigger than the width of the Middle row (152 vs 138), so the button accomodates by increasing its width to 152"),
    txt_prose("❗ padding between labels are not summed up, but a maximum is used since the point of padding is to offset against a visible element, which in this case is label's text."),
    ),
    ))//flex
     .direction(Axis::Vertical) //.main_axis_alignment(MainAxisAlignment::SpaceBetween).cross_axis_alignment(CrossAxisAlignment::Fill)
    ) //portal
}

fn main() -> Result<(), EventLoopError> {
    let app_state_init = AppState::default();
    let xapp = Xilem::new(app_state_init, app_logic).background_color(css::SEASHELL);

    let win_attr = Window::default_attributes()
        .with_title("Label: Xilem Button")
        .with_min_inner_size(LogicalSize::new(800., 600.));

    xapp.run_windowed_in(EventLoop::with_user_event(), win_attr)?;
    Ok(())
}
