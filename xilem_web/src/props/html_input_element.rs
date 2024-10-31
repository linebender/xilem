// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::modifiers::html_input_element::{Checked, DefaultChecked, Disabled, Multiple, Required};
use crate::{props, FromWithContext, Pod, PodFlags, ViewCtx};
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
    pub(crate) fn update_element(
        &mut self,
        element: &web_sys::HtmlInputElement,
        flags: &mut PodFlags,
    ) {
        let in_hydration = flags.in_hydration();

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
        self.element_props.update_element(element, flags);
    }
}

impl FromWithContext<Pod<web_sys::Element>> for Pod<web_sys::HtmlInputElement> {
    fn from_with_ctx(value: Pod<web_sys::Element>, _ctx: &mut ViewCtx) -> Self {
        Pod {
            node: value.node.unchecked_into(),
            flags: value.flags,
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

impl<T> AsMut<T> for HtmlInputElement
where
    props::Element: AsMut<T>,
{
    fn as_mut(&mut self) -> &mut T {
        self.element_props.as_mut()
    }
}

impl AsMut<Checked> for HtmlInputElement {
    fn as_mut(&mut self) -> &mut Checked {
        &mut self.checked
    }
}

impl AsMut<DefaultChecked> for HtmlInputElement {
    fn as_mut(&mut self) -> &mut DefaultChecked {
        &mut self.default_checked
    }
}

impl AsMut<Disabled> for HtmlInputElement {
    fn as_mut(&mut self) -> &mut Disabled {
        &mut self.disabled
    }
}

impl AsMut<Required> for HtmlInputElement {
    fn as_mut(&mut self) -> &mut Required {
        &mut self.required
    }
}

impl AsMut<Multiple> for HtmlInputElement {
    fn as_mut(&mut self) -> &mut Multiple {
        &mut self.multiple
    }
}

/// An alias trait to sum up all modifiers that a DOM `HTMLInputElement` can have. It's used to avoid a lot of boilerplate in public APIs.
pub trait WithHtmlInputElementProps:
    WithElementProps
    + AsMut<Checked>
    + AsMut<DefaultChecked>
    + AsMut<Disabled>
    + AsMut<Required>
    + AsMut<Multiple>
{
}
impl<
        T: WithElementProps
            + AsMut<Checked>
            + AsMut<DefaultChecked>
            + AsMut<Disabled>
            + AsMut<Required>
            + AsMut<Multiple>,
    > WithHtmlInputElementProps for T
{
}
