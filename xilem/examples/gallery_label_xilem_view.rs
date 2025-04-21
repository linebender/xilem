// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An illustration of the various options for Xilem's Label View (based on Masonry Label Widget)
// TODOs:
// add rust code generating each element in a tooltip
// add the same code in a context menu as a "copy" command
// add URL support for doc links
// add non-desktop platforms

use masonry::dpi::LogicalSize;
use winit::window::Window;
use xilem::view::{CrossAxisAlignment, FlexExt, FlexParams};

use masonry::core::ArcStr;
use masonry::parley::fontique;
use masonry::peniko::Color;
use winit::error::EventLoopError;
use xilem::view::{
    Axis, GridExt, Label, Padding, Prose, button, flex, grid, label, portal, prose, sized_box,
};
use xilem::{EventLoop, FontWeight, LineBreaking, TextAlignment, WidgetView, Xilem, palette::css};

const LABEL_COLOR: Color = css::ROYAL_BLUE;

struct AppState {
    l1i1: TextAlignment,
    l1i2: TextAlignment,
    l1i3: TextAlignment,
    l1i4: TextAlignment,
    l1i5: TextAlignment,
    l1i6: TextAlignment,
    l2i1: TextAlignment,
    l2i2: TextAlignment,
    l2i3: TextAlignment,
    l2i4: TextAlignment,
    l2i5: TextAlignment,
    l2i6: TextAlignment,
    l3i1: TextAlignment,
    l3i2: TextAlignment,
    l3i3: TextAlignment,
    l3i4: TextAlignment,
    l3i5: TextAlignment,
    l3i6: TextAlignment,
    l4i1: TextAlignment,
    l4i2: TextAlignment,
    l4i3: TextAlignment,
    l4i4: TextAlignment,
    l4i5: TextAlignment,
    l4i6: TextAlignment,
    l4i1x: CrossAxisAlignment,
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            l1i1: TextAlignment::Start,
            l1i2: TextAlignment::Left,
            l1i3: TextAlignment::Middle,
            l1i4: TextAlignment::End,
            l1i5: TextAlignment::Right,
            l1i6: TextAlignment::Justified,
            l2i1: TextAlignment::Start,
            l2i2: TextAlignment::Left,
            l2i3: TextAlignment::Middle,
            l2i4: TextAlignment::End,
            l2i5: TextAlignment::Right,
            l2i6: TextAlignment::Justified,
            l3i1: TextAlignment::Start,
            l3i2: TextAlignment::Left,
            l3i3: TextAlignment::Middle,
            l3i4: TextAlignment::End,
            l3i5: TextAlignment::Right,
            l3i6: TextAlignment::Justified,
            l4i1: TextAlignment::Start,
            l4i2: TextAlignment::Left,
            l4i3: TextAlignment::Middle,
            l4i4: TextAlignment::End,
            l4i5: TextAlignment::Right,
            l4i6: TextAlignment::Justified,
            l4i1x: CrossAxisAlignment::Start,
        }
    }
}
fn text_align_cycle(cur: &TextAlignment) -> TextAlignment {
    match cur {
        TextAlignment::Start => TextAlignment::Left,
        TextAlignment::Left => TextAlignment::Middle,
        TextAlignment::Middle => TextAlignment::End,
        TextAlignment::End => TextAlignment::Right,
        TextAlignment::Right => TextAlignment::Justified,
        TextAlignment::Justified => TextAlignment::Start,
    }
}
fn text_x_align_cycle(cur: &CrossAxisAlignment) -> CrossAxisAlignment {
    match cur {
        CrossAxisAlignment::Start => CrossAxisAlignment::Center,
        CrossAxisAlignment::Center => CrossAxisAlignment::End,
        CrossAxisAlignment::End => CrossAxisAlignment::Baseline,
        CrossAxisAlignment::Baseline => CrossAxisAlignment::Fill,
        CrossAxisAlignment::Fill => CrossAxisAlignment::Start,
    }
}


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
fn app_logic(d: &mut AppState) -> impl WidgetView<AppState> + use<> {
    let m_c = Color::from_rgb8(0x11, 0x11, 0x11); //main text
    let l_c = LABEL_COLOR;
    let mut i = 0;

    portal(
    flex((
    (txt_prose("label_gallery formats vGit@25-04 #8c25fea (in a ↕-scrollable area)").text_size(18.0),
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
    {i+=1;},title_prose(format!("§ {i} .alignment")),
    txt_prose("  6 options: ≝Start Middle End Justified Left Right\n  https://docs.rs/parley/latest/parley/layout/enum.Alignment.html")
    ),
    (label("  • flex in a 200×220 sized_box to show the impact of constraints (buttons to change alignment)").alignment(TextAlignment::Justified).brush(m_c),
    flex((
      button("Δ1/6", |da:&mut AppState|{da.l1i1 = text_align_cycle(&da.l1i1);}),
      button("Δ2/6", |da:&mut AppState|{da.l1i2 = text_align_cycle(&da.l1i2);}),
      button("Δ3/6", |da:&mut AppState|{da.l1i3 = text_align_cycle(&da.l1i3);}),
      button("Δ4/6", |da:&mut AppState|{da.l1i4 = text_align_cycle(&da.l1i4);}),
      button("Δ5/6", |da:&mut AppState|{da.l1i5 = text_align_cycle(&da.l1i5);}),
      button("Δ6/6", |da:&mut AppState|{da.l1i6 = text_align_cycle(&da.l1i6);}),
      )).direction(Axis::Horizontal),
    sized_box(
      flex((
        lc(format!("1/6 alignment {:?}",d.l1i1)).alignment(d.l1i1),
        lc(format!("2/6 alignment {:?}",d.l1i2)).alignment(d.l1i2),
        lc(format!("3/6 alignment {:?}",d.l1i3)).alignment(d.l1i3),
        lc(format!("4/6 alignment {:?}",d.l1i4)).alignment(d.l1i4),
        lc(format!("5/6 alignment {:?}",d.l1i5)).alignment(d.l1i5),
        lc(format!("6/6 alignment {:?}",d.l1i6)).alignment(d.l1i6),
      ))
      ).width(200_f64).height(220_f64).padding(Padding::from(0.))
       .background(css::LIGHT_GRAY) // .border(css::RED,0.).rounded(RoundedRectRadii::from_single_radius(0.))
    ,),
    (label("  • grid in a 200×220 sized_box to make labels same-width (one per row in a 6×1 table; buttons to change alignment)").alignment(TextAlignment::Justified).brush(m_c),
    flex((
      button("Δ1/6", |da:&mut AppState|{da.l2i1 = text_align_cycle(&da.l2i1);}),
      button("Δ2/6", |da:&mut AppState|{da.l2i2 = text_align_cycle(&da.l2i2);}),
      button("Δ3/6", |da:&mut AppState|{da.l2i3 = text_align_cycle(&da.l2i3);}),
      button("Δ4/6", |da:&mut AppState|{da.l2i4 = text_align_cycle(&da.l2i4);}),
      button("Δ5/6", |da:&mut AppState|{da.l2i5 = text_align_cycle(&da.l2i5);}),
      button("Δ6/6", |da:&mut AppState|{da.l2i6 = text_align_cycle(&da.l2i6);}),
      )).direction(Axis::Horizontal),
    sized_box(
        grid((
            lc(format!("1/6 alignment {:?}",d.l2i1)).alignment(d.l2i1).grid_pos(0,0),
            lc(format!("2/6 alignment {:?}",d.l2i2)).alignment(d.l2i2).grid_pos(0,1),
            lc(format!("3/6 alignment {:?}",d.l2i3)).alignment(d.l2i3).grid_pos(0,2),
            lc(format!("4/6 alignment {:?}",d.l2i4)).alignment(d.l2i4).grid_pos(0,3),
            lc(format!("5/6 alignment {:?}",d.l2i5)).alignment(d.l2i5).grid_pos(0,4),
            lc(format!("6/6 alignment {:?}",d.l2i6)).alignment(d.l2i6).grid_pos(0,5),
            ),1,6,).spacing(0.0)
        ).width(200_f64).height(220_f64).padding(Padding::from(0.))
         .background(css::LIGHT_GRAY) //.border(css::RED,0.).rounded(RoundedRectRadii::from_single_radius(0.))
    ,),
    (label("  • unboxed (constrained by root parent's flex in a portal)\n  (Start=Middle: parent Flex container ≝CrossAxisAlignment::Center,\n  so the alignment for a label starts at the center)").alignment(TextAlignment::Justified).brush(m_c),
    label("  can be fixed with a custom per-element override .flex(FlexParams::new(1.0,CrossAxisAlignment::Start)) (buttons to change alignment)").alignment(TextAlignment::Justified).brush(m_c),
    flex((
      button("Δ1/6", |da:&mut AppState|{da.l3i1 = text_align_cycle(&da.l3i1);}),
      button("Δ2/6", |da:&mut AppState|{da.l3i2 = text_align_cycle(&da.l3i2);}),
      button("Δ3/6", |da:&mut AppState|{da.l3i3 = text_align_cycle(&da.l3i3);}),
      button("Δ4/6", |da:&mut AppState|{da.l3i4 = text_align_cycle(&da.l3i4);}),
      button("Δ5/6", |da:&mut AppState|{da.l3i5 = text_align_cycle(&da.l3i5);}),
      button("Δ6/6", |da:&mut AppState|{da.l3i6 = text_align_cycle(&da.l3i6);}),
      )).direction(Axis::Horizontal),
    lc(format!("1/6 alignment {:?}",d.l3i1)).alignment(d.l3i1),
    lc(format!("2/6 alignment {:?}",d.l3i2)).alignment(d.l3i2),
    lc(format!("3/6 alignment {:?}",d.l3i3)).alignment(d.l3i3),
    lc(format!("4/6 alignment {:?}",d.l3i4)).alignment(d.l3i4),
    lc(format!("5/6 alignment {:?}",d.l3i5)).alignment(d.l3i5),
    lc(format!("6/6 alignment {:?}",d.l3i6)).alignment(d.l3i6),
    ),
    (label("  • flex in a 500×200 sized_box (buttons to change alignment)").alignment(TextAlignment::Justified).brush(m_c),
    txt_prose("  5 cross-alignment options: Start Center End Baseline Fill https://docs.rs/masonry/latest/masonry/widget/enum.CrossAxisAlignment.html"),
    flex((
      button("Δ1/6", |da:&mut AppState|{da.l4i1 = text_align_cycle(&da.l4i1);}),
      button("Δ1/6 cross", |da:&mut AppState|{da.l4i1x = text_x_align_cycle(&da.l4i1x)}),
      button("Δ2/6", |da:&mut AppState|{da.l4i2 = text_align_cycle(&da.l4i2);}),
      button("Δ3/6", |da:&mut AppState|{da.l4i3 = text_align_cycle(&da.l4i3);}),
      button("Δ4/6", |da:&mut AppState|{da.l4i4 = text_align_cycle(&da.l4i4);}),
      button("Δ5/6", |da:&mut AppState|{da.l4i5 = text_align_cycle(&da.l4i5);}),
      button("Δ6/6", |da:&mut AppState|{da.l4i6 = text_align_cycle(&da.l4i6);}),
      )).direction(Axis::Horizontal),
    sized_box(flex((
        lc(format!("1/6 alignment {:?}",d.l4i1)).alignment(d.l4i1),
        lc(format!("1/6 alignment {:?} + CrossAxisAlignment {:?}",d.l4i1,d.l4i1x)).alignment(d.l4i1).flex(FlexParams::new(1.0,d.l4i1x)),
        lc(format!("2/6 alignment {:?}",d.l4i2)).alignment(d.l4i2),
        lc(format!("3/6 alignment {:?}",d.l4i3)).alignment(d.l4i3),
        lc(format!("4/6 alignment {:?}",d.l4i4)).alignment(d.l4i4),
        lc(format!("5/6 alignment {:?}",d.l4i5)).alignment(d.l4i5),
        lc(format!("6/6 alignment {:?}",d.l4i6)).alignment(d.l4i6),
        ))
        ).width(500_f64).height(200_f64).padding(Padding::from(0.))
         .background(css::LIGHT_GRAY) //.border(css::RED,0.).rounded(RoundedRectRadii::from_single_radius(0.))
    ),
    {i+=1;},(title_prose(format!("§ {i} .line_break_mode")),
    txt_prose("  3 options: ≝WordWrap Clip Overflow\n  https://docs.rs/masonry/latest/masonry/widget/enum.LineBreaking.html"),
    label("  • grid in a 340×120 box to make labels same-width (one per row in a 3×1 table)").alignment(TextAlignment::Justified).brush(m_c),
    sized_box(
        grid((
            lc("1/3 WordWrap: break at word boundaries = abcd-efgh-ijkl-mnop-qrst-uvwx-yz"  ).line_break_mode(LineBreaking::WordWrap    ).grid_pos(0,0),
            lc("2/3 Clip: truncate to label's width = abcd-efgh-ijkl-mnop-qrst-uvwx-yz" ).line_break_mode(LineBreaking::Clip        ).grid_pos(0,1),
            lc("3/3 Overflow: overflow the label = abcd-efgh-ijkl-mnop-qrst-uvwx-yz"        ).line_break_mode(LineBreaking::Overflow    ).grid_pos(0,2),
            ),1,3,).spacing(0.0)
        ).width(340_f64).height(120_f64).padding(Padding::from(0.))
         .background(css::LIGHT_GRAY)
    ),

    (title_prose(format!("§ {i}a .font")),
    txt_prose(" (5 examples, some might be invisible due to missing fonts. 🐞❓font substitution?)"),
    flex((
        (lc("1Times New Roman"  ).font("Times New Roman"    ),
        lc("2Arial"             ).font("Arial"              ),
        lc("3Cambria"           ).font("Cambria"            ),
        lc("4Cambria Math"      ).font("Cambria Math"       ),
        lc("5Verdana"           ).font("Verdana"            ),
        ),
    )).direction(Axis::Horizontal),
    {i+=1;},title_prose(format!("§ {i}b .font(fontique::GenericFamily::↓)")),
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

    {i+=1;},(title_prose(format!("§ {i} Unsupported Masonry options")),
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
        .with_title("label_gallery")
        .with_min_inner_size(LogicalSize::new(800., 600.));

    xapp.run_windowed_in(EventLoop::with_user_event(), win_attr)?;
    Ok(())
}
