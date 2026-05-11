// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Stress test for the build speed of a Xilem component with lots of styles applied.

use std::hint::black_box;

use xilem::WidgetView;
use xilem::masonry::layout::AsUnit;
use xilem::style::Style as _;
use xilem::view::prose;

#[cfg(compile_stress_test)]
fn prop_stack() -> impl WidgetView<()> + use<> {
    prose("")
        .border_width(0.5.px())
        .border_width(0.5.px())
        .border_width(0.5.px())
        .border_width(0.5.px())
        .border_width(0.5.px())
        .border_width(0.5.px())
        .border_width(0.5.px())
        .border_width(0.5.px())
        .border_width(0.5.px())
}

#[cfg(not(compile_stress_test))]
fn prop_stack() -> impl WidgetView<()> + use<> {
    prose("")
        // We use only one to check it compiles
        .border_width(0.5.px())
}

#[test]
fn test() {
    black_box(prop_stack().boxed());
}
