// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::layout::{LenDef, Length};

/// Specifies how a widget dimension's length is derived.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum Dim {
    /// Automatically determine a reasonable length with a strategy chosen by the container widget.
    ///
    /// This may result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    #[default]
    Auto,
    /// Specific fixed length.
    ///
    /// The value is in logical pixels, represented by a [`Length`].
    Fixed(Length),
    /// Multiple of context length.
    ///
    /// For example, `Ratio(0.5)` will result in 50% of the context length.
    ///
    /// Context length is usually the container widget's length excluding its borders and padding.
    /// Examples of exceptions include `Grid` which will provide the child's area length,
    /// i.e. the sum of cell lengths that the child occupies, and `Portal` which will provide
    /// its viewport length.
    ///
    /// If there is no context length, e.g. the container hasn't calculated its dynamic length yet,
    /// then `Ratio` will fall back to [`Auto`].
    ///
    /// The ratio value must be finite and non-negative.
    ///
    /// [`Auto`]: Self::Auto
    Ratio(f64),
    /// Mimics the context length.
    ///
    /// Essentially a shorthand for [`Ratio(1.)`].
    ///
    /// [`Ratio(1.)`]: Self::Ratio
    Stretch,
    /// Intrinsic minimum length.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    MinContent,
    /// Intrinsic preferred length.
    ///
    /// This will result in a [`measure`] invocation, which can be slow.
    ///
    /// [`measure`]: crate::core::Widget::measure
    MaxContent,
    /// The widget should attempt to fit into the context length.
    ///
    /// If there is no context length, e.g. the container hasn't calculated its dynamic length yet,
    /// then `FitContent` will fall back to [`Auto`].
    ///
    /// This may result in a [`measure`] invocation, which can be slow.
    ///
    /// [`Auto`]: Self::Auto
    /// [`measure`]: crate::core::Widget::measure
    FitContent,
}

impl From<Length> for Dim {
    fn from(value: Length) -> Self {
        Self::Fixed(value)
    }
}

impl Dim {
    /// Resolves, if possible, into a [`LenDef`].
    ///
    /// If `context_length` is provided, it must be in device pixels.
    pub fn resolve(&self, scale: f64, context_length: Option<f64>) -> Option<LenDef> {
        match self {
            Self::Fixed(length) => Some(LenDef::Fixed(length.dp(scale))),
            Self::Ratio(mul) => context_length.map(|cl| LenDef::Fixed(cl * *mul)),
            Self::Stretch => context_length.map(LenDef::Fixed),
            Self::MinContent => Some(LenDef::MinContent),
            Self::MaxContent => Some(LenDef::MaxContent),
            Self::FitContent => context_length.map(LenDef::FitContent),
            Self::Auto => None,
        }
    }
}
