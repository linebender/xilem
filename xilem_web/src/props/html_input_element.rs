// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::modifiers::html_input_element::{Checked, DefaultChecked, Disabled, Multiple, Required};
use crate::modifiers::{Modifier, WithModifier};
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
    fn from_with_ctx(value: Pod<web_sys::Element>, _ctx: &mut ViewCtx) -> Self {
        Pod {
            node: value.node.unchecked_into(),
            props: HtmlInputElement {
                checked: Checked::default(),
                default_checked: DefaultChecked::default(),
                disabled: Disabled::default(),
                required: Required::default(),
                multiple: Multiple::default(),
                element_props: value.props,
            },
        }
    }
}

impl<T> WithModifier<T> for HtmlInputElement
where
    props::Element: WithModifier<T>,
{
    fn modifier(&mut self) -> Modifier<'_, T> {
        self.element_props.modifier()
    }
}

impl WithModifier<Checked> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Checked> {
        Modifier::new(&mut self.checked, &mut self.element_props.flags)
    }
}

impl WithModifier<DefaultChecked> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, DefaultChecked> {
        Modifier::new(&mut self.default_checked, &mut self.element_props.flags)
    }
}

impl WithModifier<Disabled> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Disabled> {
        Modifier::new(&mut self.disabled, &mut self.element_props.flags)
    }
}

impl WithModifier<Required> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Required> {
        Modifier::new(&mut self.required, &mut self.element_props.flags)
    }
}

impl WithModifier<Multiple> for HtmlInputElement {
    fn modifier(&mut self) -> Modifier<'_, Multiple> {
        Modifier::new(&mut self.multiple, &mut self.element_props.flags)
    }
}

/// An alias trait to sum up all modifiers that a DOM `HTMLInputElement` can have. It's used to avoid a lot of boilerplate in public APIs.
pub trait WithHtmlInputElementProps:
    WithElementProps
    + WithModifier<Checked>
    + WithModifier<DefaultChecked>
    + WithModifier<Disabled>
    + WithModifier<Required>
    + WithModifier<Multiple>
{
}
impl<
        T: WithElementProps
            + WithModifier<Checked>
            + WithModifier<DefaultChecked>
            + WithModifier<Disabled>
            + WithModifier<Required>
            + WithModifier<Multiple>,
    > WithHtmlInputElementProps for T
{
}
