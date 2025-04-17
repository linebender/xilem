// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An illustration of the various options for Xilem's Label View (based on Masonry Label Widget)
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
use xilem::view::{CrossAxisAlignment, FlexExt, FlexParams};

use masonry::core::ArcStr;
use masonry::parley::fontique;
use masonry::peniko::Color;
use winit::error::EventLoopError;
use xilem::view::{
    flex, grid, label, portal, prose, sized_box, Axis, GridExt, Label, Padding, Prose,
};
use xilem::{palette::css, EventLoop, FontWeight, LineBreaking, TextAlignment, WidgetView, Xilem};

const LABEL_COLOR: Color = css::ROYAL_BLUE;

#[derive(Default)]
struct AppState {}

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
fn lc(text: impl Into<ArcStr>) -> Label {
    //colored label
    label(text).brush(LABEL_COLOR)
}
fn app_logic(_d: &mut AppState) -> impl WidgetView<AppState> {
    let m_c = Color::from_rgb8(0x11, 0x11, 0x11); //main text
    let l_c = LABEL_COLOR;
    let mut i = 1;

    portal(
    flex((
    (txt_prose("Xilem view::Label formats vGit@25-02 #25b12ad (in a ↕-scrollable area)").text_size(18.0),
    if cfg!(debug_assertions) {txt_prose(
     "This is a debug build, so you can use github.com/linebender/xilem/tree/main/masonry#debugging-features:
        • F11 to toggle a rudimentary widget inspector
        • F12 to toggle paint widget layout rectangles")
    } else {txt_prose("This is not a debug build, so F11 widget inspector and F12 widget rectangles tools are not available)\ngithub.com/linebender/xilem/tree/main/masonry#debugging-features")},
    label(format!("Label: Serif Bold 14 {LABEL_COLOR:?}")).text_size(14.0).weight(FontWeight::BOLD) // float bold=700, FontWeight::parse("normal") for css
        .font(fontique::GenericFamily::Serif)
        .alignment(TextAlignment::Start)
        .brush(l_c) //impl Into<peniko:Brush> brush sets text color, but gradients and images are also supported Solid(color::AlphaColor<Srgb>) Gradient(Gradient) Image(Image),
        .line_break_mode(LineBreaking::Overflow) //WordWrap Clip Overflow
        ,
    title_prose(format!("§ {i} .alignment")),{i+=1;},
    txt_prose("  4 options: ≝Start Middle End Justified\n  https://docs.rs/parley/latest/parley/layout/enum.Alignment.html")
    ),
    // doesn't seem to be different now vs unconstrained
    // (lc("  •flex in a 200×70 box to show impact of constraints ").alignment(TextAlignment::Start),
    // sized_box(
    //   flex((
    //     lc("1/4 alignment Start"     ).alignment(TextAlignment::Start        ),
    //     lc("2/4 alignment Middle"    ).alignment(TextAlignment::Middle       ),
    //     lc("3/4 alignment End"       ).alignment(TextAlignment::End          ),
    //     lc("4/4 alignment Justified" ).alignment(TextAlignment::Justified    ),
    //   ))
    //   ).width(200f64).height(70f64).padding(Padding::from(0.))
    //    .background(css::LIGHT_GRAY) // .border(css::RED,0.).rounded(RoundedRectRadii::from_single_radius(0.))
    // ,),
    (label("  • grid in a 200×70 sized_box to make labels same-width (one per row in 4×1 table)").alignment(TextAlignment::Justified).brush(m_c),
    sized_box(
        grid((
            lc("1/4 alignment Start"        ).alignment(TextAlignment::Start        ).grid_pos(0,0),
            lc("2/4 alignment Middle"       ).alignment(TextAlignment::Middle       ).grid_pos(0,1),
            lc("3/4 alignment End"          ).alignment(TextAlignment::End          ).grid_pos(0,2),
            lc("4/4 alignment Justified"    ).alignment(TextAlignment::Justified    ).grid_pos(0,3),
            ),1,4,).spacing(0.0)
        ).width(200_f64).height(70_f64).padding(Padding::from(0.))
         .background(css::LIGHT_GRAY) //.border(css::RED,0.).rounded(RoundedRectRadii::from_single_radius(0.))
    ,),
    (label("  • unboxed (constrained by root parent's flex in a portal)\n  (Start=Middle: parent Flex container ≝CrossAxisAlignment::Center,\n  so the alignment for a label starts at the center)").alignment(TextAlignment::Justified).brush(m_c),
    label("  can be fixed with a custom per-element override .flex(FlexParams::new(1.0,CrossAxisAlignment::Start))").alignment(TextAlignment::Justified).brush(m_c),
    lc("1/4 alignment Start"        ).alignment(TextAlignment::Start        ),
    lc("2/4 alignment Middle"       ).alignment(TextAlignment::Middle       ),
    lc("3/4 alignment End"          ).alignment(TextAlignment::End          ),
    lc("4/4 alignment Justified"    ).alignment(TextAlignment::Justified    ),
    ),
    (label("  • flex in a 500×140 sized_box (🐞? unboxed .flex override removes Portal scrolling)").alignment(TextAlignment::Justified).brush(m_c),
    sized_box(flex((
        lc("1/4 alignment Start"                                ).alignment(TextAlignment::Start        ),
        lc("1/4 alignment Start + CrossAxisAlignment::Start "   ).alignment(TextAlignment::Start        ).flex(FlexParams::new(1.0,CrossAxisAlignment::Start)),
        lc("2/4 alignment Middle"                               ).alignment(TextAlignment::Middle       ),
        lc("3/4 alignment End"                                  ).alignment(TextAlignment::End          ),
        lc("4/4 alignment Justified"                            ).alignment(TextAlignment::Justified    ),
        ))
        ).width(500_f64).height(140_f64).padding(Padding::from(0.))
         .background(css::LIGHT_GRAY) //.border(css::RED,0.).rounded(RoundedRectRadii::from_single_radius(0.))
    ),
    (title_prose(format!("§ {i} .line_break_mode")),{i+=1;},
    txt_prose("  3 options: ≝WordWrap Clip Overflow\n  https://docs.rs/masonry/latest/masonry/widget/enum.LineBreaking.html"),
    label("  • grid in a 340×120 box to make labels same-width (one per row in 3×1 table)").alignment(TextAlignment::Justified).brush(m_c),
    sized_box(
        grid((
            lc("1/3 WordWrap: break at word boundaries = abcd-efgh-ijkl-mnop-qrst-uvwx-yz"  ).line_break_mode(LineBreaking::WordWrap    ).grid_pos(0,0),
            lc("2/3 Clip    : truncate to label's width = abcd-efgh-ijkl-mnop-qrst-uvwx-yz" ).line_break_mode(LineBreaking::Clip        ).grid_pos(0,1),
            lc("3/3 Overflow: overflow the label = abcd-efgh-ijkl-mnop-qrst-uvwx-yz"        ).line_break_mode(LineBreaking::Overflow    ).grid_pos(0,2),
            ),1,3,).spacing(0.0)
        ).width(340_f64).height(120_f64).padding(Padding::from(0.))
         .background(css::LIGHT_GRAY)
    ),

    (title_prose(format!("§ {i}a .font")),
    txt_prose(" (some options might be invisible due to missing fonts. 🐞❓font substitution?)"),
    flex((
        (lc("1Times New Roman"  ).font("Times New Roman"    ),
        lc("2Arial"             ).font("Arial"              ),
        lc("3Cambria"           ).font("Cambria"            ),
        lc("4Cambria Math"      ).font("Cambria Math"       ),
        lc("5Verdana"           ).font("Verdana"            ),
        ),
    )).direction(Axis::Horizontal),
    title_prose(format!("§ {i}b .font(fontique::GenericFamily::↓)")),{i+=1;},
    flex((
        lc("1Serif"     ).font(fontique::GenericFamily::Serif       ),
        lc("2SansSerif" ).font(fontique::GenericFamily::SansSerif   ),
        lc("3Monospace" ).font(fontique::GenericFamily::Monospace   ),
        lc("4Cursive"   ).font(fontique::GenericFamily::Cursive     ),
        lc("5Fantasy"   ).font(fontique::GenericFamily::Fantasy     ),
    )).direction(Axis::Horizontal),
    flex((
        lc("6SystemUi"      ).font(fontique::GenericFamily::SystemUi    ),
        lc("7UiSerif"       ).font(fontique::GenericFamily::UiSerif     ),
        lc("8UiSansSerif"   ).font(fontique::GenericFamily::UiSansSerif ),
        lc("9UiMonospace"   ).font(fontique::GenericFamily::UiMonospace ),
        lc("10UiRounded"    ).font(fontique::GenericFamily::UiRounded   ),
    )).direction(Axis::Horizontal),
    flex((
        lc("11Emoji"        ).font(fontique::GenericFamily::Emoji       ),
        lc("12Math→"        )                                            ,
        lc("⊂⊃Ψ⌈Δ∇∵ℕ⇑⇑₇∞"  ).font(fontique::GenericFamily::Math        ),
        lc("13FangSong"     ).font(fontique::GenericFamily::FangSong    ),
    )).direction(Axis::Horizontal),
    ),

    (title_prose(format!("§ {i} Unsupported Masonry options")),{i+=1;},
    txt_prose("  hinting, disabled color, styles (underline, strikethrough, word/letter spacing, font features etc. https://docs.rs/parley/latest/parley/style/enum.StyleProperty.html)"),
    ),
    ))//flex
     .direction(Axis::Vertical) //.main_axis_alignment(MainAxisAlignment::SpaceBetween).cross_axis_alignment(CrossAxisAlignment::Fill)
    ) //portal
}

fn main() -> Result<(), EventLoopError> {
    let app_state_init = AppState::default();
    let xapp = Xilem::new(app_state_init, app_logic).background_color(css::SEASHELL);

    let win_attr = Window::default_attributes()
        .with_title("Label: Xilem View")
        .with_min_inner_size(LogicalSize::new(800., 600.));

    xapp.run_windowed_in(EventLoop::with_user_event(), win_attr)?;
    Ok(())
}
