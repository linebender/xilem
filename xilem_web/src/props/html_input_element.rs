// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::modifiers::html_input_element::{Checked, DefaultChecked, Disabled, Multiple, Required};
use crate::modifiers::{Modifier, With};
use crate::{props, FromWithContext, Pod, ViewCtx};
use wasm_bindgen::JsCast as _;

use super::WithElementProps;

/// Props specific to an input element.
pub struct HtmlInputElement {
    element_props: props::Element,
    checked: Checked,
    default_checked: DefaultChecked,
    disabled: Disabled,
    required: Required,
    multiple: Multiple,
}

impl HtmlInputElement {
    pub(crate) fn update_element(&mut self, element: &web_sys::HtmlInputElement) {
        if self.element_props.flags.needs_update() {
            let in_hydration = self.element_props.flags.in_hydration();

            // Set booleans to `false` as this is the default,
            // if we wouldn't do that, possibly the previous value would persist, which is likely unwanted.
            self.checked.apply_changes(|value| {
                if !in_hydration {
                    element.set_checked(value.unwrap_or(false));
                }
            });
            self.default_checked.apply_changes(|value| {
                if !in_hydration {
                    element.set_default_checked(value.unwrap_or(false));
                }
            });
            self.disabled.apply_changes(|value| {
                if !in_hydration {
                    element.set_disabled(value.unwrap_or(false));
                }
            });
            self.required.apply_changes(|value| {
                if !in_hydration {
                    element.set_required(value.unwrap_or(false));
                }
            });
            self.multiple.apply_changes(|value| {
                if !in_hydration {
                    element.set_multiple(value.unwrap_or(false));
                }
            });
        }
        // flags are cleared in the following call
        self.element_props.update_element(element);
    }
}

impl FromWithContext<Pod<web_sys::Element>> for Pod<web_sys::HtmlInputElement> {
    fn from_with_ctx(value: Pod<web_sys::Element>, ctx: &mut ViewCtx) -> Self {
        let checked_size_hint = ctx.take_modifier_size_hint::<Checked>();
        let default_checked_size_hint = ctx.take_modifier_size_hint::<DefaultChecked>();
        let disabled_size_hint = ctx.take_modifier_size_hint::<Disabled>();
        let required_size_hint = ctx.take_modifier_size_hint::<Required>();
        let multiple_size_hint = ctx.take_modifier_size_hint::<Multiple>();
        Pod {
            node: value.node.unchecked_into(),
            props: HtmlInputElement {
                checked: Checked::new(checked_size_hint),
                default_checked: DefaultChecked::new(default_checked_size_hint),
                disabled: Disabled::new(disabled_size_hint),
                required: Required::new(required_size_hint),
                multiple: Multiple::new(multiple_size_hint),
                element_props: value.props,
            },
        }
    }
}

impl<T> With<T> for HtmlInputElement
where
    props::Element: With<T>,
{
    fn modifier(&mut self) -> Modifier<'_, T> {
        self.element_props.modifier()
    }
}

impl With<Checked> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Checked> {
        Modifier::new(&mut self.checked, &mut self.element_props.flags)
    }
}

impl With<DefaultChecked> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, DefaultChecked> {
        Modifier::new(&mut self.default_checked, &mut self.element_props.flags)
    }
}

impl With<Disabled> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Disabled> {
        Modifier::new(&mut self.disabled, &mut self.element_props.flags)
    }
}

impl With<Required> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Required> {
        Modifier::new(&mut self.required, &mut self.element_props.flags)
    }
}

impl With<Multiple> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Multiple> {
        Modifier::new(&mut self.multiple, &mut self.element_props.flags)
    }
}

pub trait WithHtmlInputElementProps:
    WithElementProps
    + With<Checked>
    + With<DefaultChecked>
    + With<Disabled>
    + With<Required>
    + With<Multiple>
{
}
impl<
        T: WithElementProps
            + With<Checked>
            + With<DefaultChecked>
            + With<Disabled>
            + With<Required>
            + With<Multiple>,
    > WithHtmlInputElementProps for T
{
}
