// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! This example uses variable fonts in a touch sensitive digital clock.

use std::sync::Arc;
use std::time::Duration;

use masonry::properties::types::AsUnit;
use time::error::IndeterminateOffset;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};
use winit::error::EventLoopError;
use xilem::core::fork;
use xilem::style::Style as _;
use xilem::view::{
    FlexExt, FlexSpacer, button, flex, flex_row, inline_prose, label, portal, prose, sized_box,
    task, variable_label,
};
use xilem::{
    Blob, EventLoop, EventLoopBuilder, FontWeight, WidgetView, WindowOptions, Xilem, palette,
};

/// The state of the application, owned by Xilem and updated by the callbacks below.
struct Clocks {
    /// The font [weight](FontWeight) used for the values.
    weight: f32,
    /// The current UTC offset on this machine.
    local_offset: Result<UtcOffset, IndeterminateOffset>,
    /// The current time.
    now_utc: OffsetDateTime,
}

/// A possible timezone, with an offset from UTC.
struct TimeZone {
    /// An approximate region which this offset applies to.
    region: &'static str,
    /// The offset from UTC
    offset: UtcOffset,
}

fn app_logic(data: &mut Clocks) -> impl WidgetView<Clocks> + use<> {
    let view = flex((
        // HACK: We add a spacer at the top for Android. See https://github.com/rust-windowing/winit/issues/2308
        FlexSpacer::Fixed(40.px()),
        local_time(data),
        controls(),
        portal(flex(
            // TODO: When we get responsive layouts, move this into a two-column view on desktop.
            TIMEZONES.iter().map(|it| it.view(data)).collect::<Vec<_>>(),
        ))
        .flex(1.),
    ))
    .padding(10.0);
    fork(
        view,
        task(
            |proxy| async move {
                // TODO: Synchronise with the actual "second" interval. This is expected to show the wrong second
                // ~50% of the time.
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    let Ok(()) = proxy.message(()) else {
                        break;
                    };
                }
            },
            |data: &mut Clocks, ()| data.now_utc = OffsetDateTime::now_utc(),
        ),
    )
}

/// Shows the current system time on a best-effort basis.
// TODO: Maybe make this have a larger font size?
fn local_time(data: &mut Clocks) -> impl WidgetView<Clocks> + use<> {
    let (error_view, offset) = if let Ok(offset) = data.local_offset {
        (None, offset)
    } else {
        (
            Some(
                prose("Could not determine local UTC offset, using UTC")
                    .text_color(palette::css::ORANGE_RED),
            ),
            UtcOffset::UTC,
        )
    };

    flex((
        TimeZone {
            region: "Here",
            offset,
        }
        .view(data),
        error_view,
    ))
}

/// Controls for the variable font weight.
fn controls() -> impl WidgetView<Clocks> {
    flex_row((
        button("Increase", |data: &mut Clocks| {
            data.weight = (data.weight + 100.).clamp(1., 1000.);
        }),
        button("Decrease", |data: &mut Clocks| {
            data.weight = (data.weight - 100.).clamp(1., 1000.);
        }),
        button("Minimum", |data: &mut Clocks| {
            data.weight = 1.;
        }),
        button("Maximum", |data: &mut Clocks| {
            data.weight = 1000.;
        }),
    ))
}

impl TimeZone {
    /// Display this timezone as a row, designed to be shown in a list of time zones.
    fn view(&self, data: &mut Clocks) -> impl WidgetView<Clocks> + use<> {
        let date_time_in_self = data.now_utc.to_offset(self.offset);
        sized_box(flex((
            flex_row((
                inline_prose(self.region),
                FlexSpacer::Flex(1.),
                label(format!("UTC{}", self.offset)).color(
                    if data.local_offset.is_ok_and(|it| it == self.offset) {
                        // TODO: Consider accessibility here.
                        palette::css::ORANGE
                    } else {
                        masonry::theme::TEXT_COLOR
                    },
                ),
            ))
            .must_fill_major_axis(true)
            .flex(1.),
            flex_row((
                variable_label(
                    date_time_in_self
                        .format(format_description!("[hour repr:24]:[minute]:[second]"))
                        .unwrap()
                        .to_string(),
                )
                .text_size(48.)
                // Use the roboto flex we have just loaded.
                .font("Roboto Flex")
                .target_weight(data.weight, 400.),
                FlexSpacer::Flex(1.0),
                (data.local_now().date() != date_time_in_self.date()).then(|| {
                    label(
                        date_time_in_self
                            .format(format_description!("([day] [month repr:short])"))
                            .unwrap(),
                    )
                }),
            )),
        )))
        .expand_width()
        .height(72.px())
    }
}

impl Clocks {
    fn local_now(&self) -> OffsetDateTime {
        match self.local_offset {
            Ok(offset) => self.now_utc.to_offset(offset),
            Err(_) => self.now_utc,
        }
    }
}

/// A subset of [Roboto Flex](https://fonts.google.com/specimen/Roboto+Flex), used under the OFL.
/// This is a variable font, and so can have its axes be animated.
/// The version in the repository supports the numbers 0-9 and `:`, to this examples use of
/// it for clocks.
/// Full details can be found in `xilem/resources/fonts/roboto_flex/README` from
/// the workspace root.
const ROBOTO_FLEX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/fonts/roboto_flex/",
    // The full font file is *not* included in this repository, due to size constraints.
    // If you download the full font, you can use it by moving it into the roboto_flex folder,
    // then swapping which of the following two lines is commented out:
    // "RobotoFlex-VariableFont_GRAD,XOPQ,XTRA,YOPQ,YTAS,YTDE,YTFI,YTLC,YTUC,opsz,slnt,wdth,wght.ttf",
    "RobotoFlex-Subset.ttf"
));

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = Clocks {
        weight: FontWeight::BLACK.value(),
        // TODO: We can't get this on Android, because
        local_offset: UtcOffset::current_local_offset(),
        now_utc: OffsetDateTime::now_utc(),
    };

    // Load Roboto Flex so that it can be used at runtime.
    let app = Xilem::new_simple(data, app_logic, WindowOptions::new("Clocks"))
        .with_font(Blob::new(Arc::new(ROBOTO_FLEX)));

    app.run_in(event_loop)?;
    Ok(())
}

/// A shorthand for creating a [`TimeZone`].
const fn tz(region: &'static str, offset: i8) -> TimeZone {
    TimeZone {
        region,
        offset: match UtcOffset::from_hms(offset, 0, 0) {
            Ok(it) => it,
            Err(_) => {
                panic!("Component out of range.");
            }
        },
    }
}

/// A static list of timezones to display. All regions selected do not observe daylight savings time.
///
/// The timezones were determined on 2024-08-14.
const TIMEZONES: &[TimeZone] = &[
    tz("Hawaii", -10),
    tz("Pitcairn Islands", -8),
    tz("Arizona", -7),
    tz("Saskatchewan", -6),
    tz("Peru", -5),
    tz("Barbados", -4),
    tz("Martinique", -4),
    tz("Uruguay", -3),
    tz("Iceland", 0),
    tz("Tunisia", 1),
    tz("Mozambique", 2),
    tz("Qatar", 3),
    tz("Azerbaijan", 4),
    tz("Pakistan", 5),
    tz("Bangladesh", 6),
    tz("Thailand", 7),
    tz("Singapore", 8),
    tz("Japan", 9),
    tz("Queensland", 10),
    tz("Tonga", 13),
];

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::builder())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::builder();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
