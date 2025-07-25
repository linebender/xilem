// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{BrushIndex, Property, UpdateCtx};

macro_rules! impl_text_property {
    (
        $(
            $docstring:literal
            $Prop:ident($ty:ty) = $default:expr
        ,)*
    ) => {
        $(
            #[doc = $docstring]
            ///
            /// **IMPORTANT:** This property is only defined for [`Label`] and [`TextArea`], *not*
            /// for widgets embedding them such as [`Button`], [`Checkbox`], [`TextInput`], [`Prose`], etc.
            ///
            /// [`Label`]: crate::widgets::Label
            /// [`TextArea`]: crate::widgets::TextArea
            /// [`Button`]: crate::widgets::Button
            /// [`Checkbox`]: crate::widgets::Checkbox
            /// [`TextInput`]: crate::widgets::TextInput
            /// [`Prose`]: crate::widgets::Prose
            #[derive(Clone, Debug, PartialEq)]
            pub struct $Prop($ty);

            // ---

            impl Property for $Prop {
                fn static_default() -> &'static Self {
                    static DEFAULT: $Prop = $Prop($default);
                    &DEFAULT
                }
            }

            impl Default for $Prop {
                fn default() -> Self {
                    Self::static_default().clone()
                }
            }

            impl $Prop {
                /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
                pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
                    if property_type != TypeId::of::<Self>() {
                        return;
                    }
                    ctx.request_layout();
                }
            }
        )*
    };
}

impl_text_property! {
    "Font family stack."
    FontStack(parley::FontStack<'static>) = parley::FontStack::List(std::borrow::Cow::Borrowed(&[])),

    "Font size."
    FontSize(f32) = 12.0,

    "Font width."
    FontWidth(parley::FontWidth) = parley::FontWidth::NORMAL,

    "Font style."
    FontStyle(parley::FontStyle) = parley::FontStyle::Normal,

    "Font weight."
    FontWeight(parley::FontWeight) = parley::FontWeight::NORMAL,

    "Font variation settings."
    FontVariations(parley::FontSettings<'static, parley::FontVariation>) = parley::FontSettings::List(std::borrow::Cow::Borrowed(&[])),

    "Font feature settings."
    FontFeatures(parley::FontSettings<'static, parley::FontFeature>) = parley::FontSettings::List(std::borrow::Cow::Borrowed(&[])),

    "Locale."
    Locale(Option<&'static str>) = None,

    "Underline decoration."
    Underline(bool) = false,

    "Offset of the underline decoration."
    UnderlineOffset(Option<f32>) = None,

    "Size of the underline decoration."
    UnderlineSize(Option<f32>) = None,

    "Brush for rendering the underline decoration."
    UnderlineBrush(Option<BrushIndex>) = None,

    "Strikethrough decoration."
    Strikethrough(bool) = false,

    "Offset of the strikethrough decoration."
    StrikethroughOffset(Option<f32>) = None,

    "Size of the strikethrough decoration."
    StrikethroughSize(Option<f32>) = None,

    "Brush for rendering the strikethrough decoration."
    StrikethroughBrush(Option<BrushIndex>) = None,

    "Line height."
    LineHeight(parley::LineHeight) = parley::LineHeight::MetricsRelative(1.0),

    "Extra spacing between words."
    WordSpacing(f32) = 0.,

    "Extra spacing between letters."
    LetterSpacing(f32) = 0.,

    "Control over where words can wrap."
    WordBreak(parley::WordBreakStrength) = parley::WordBreakStrength::Normal,

    "Control over \"emergency\" line-breaking."
    OverflowWrap(parley::OverflowWrap) = parley::OverflowWrap::Normal,

}
