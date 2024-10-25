use crate::modifiers::html_input_element::{Checked, DefaultChecked, Disabled, Multiple, Required};
use crate::modifiers::With;
use crate::{props, FromWithContext, Pod, ViewCtx};
use wasm_bindgen::JsCast as _;

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
        self.element_props.update_element(element);
        // Set booleans to `false` as this is the default,
        // if we wouldn't do that, possibly the previous value would persist, which is likely unwanted.
        self.checked.apply_changes(|in_hydration, value| {
            if !in_hydration {
                element.set_checked(value.unwrap_or(false));
            }
        });
        self.default_checked.apply_changes(|in_hydration, value| {
            if !in_hydration {
                element.set_default_checked(value.unwrap_or(false));
            }
        });
        self.disabled.apply_changes(|in_hydration, value| {
            if !in_hydration {
                element.set_disabled(value.unwrap_or(false));
            }
        });
        self.required.apply_changes(|in_hydration, value| {
            if !in_hydration {
                element.set_required(value.unwrap_or(false));
            }
        });
        self.multiple.apply_changes(|in_hydration, value| {
            if !in_hydration {
                element.set_multiple(value.unwrap_or(false));
            }
        });
    }
}

impl FromWithContext<Pod<web_sys::Element>> for Pod<web_sys::HtmlInputElement> {
    fn from_with_ctx(value: Pod<web_sys::Element>, ctx: &mut ViewCtx) -> Self {
        let checked_size_hint = ctx.take_modifier_size_hint::<Checked>();
        let default_checked_size_hint = ctx.take_modifier_size_hint::<DefaultChecked>();
        let disabled_size_hint = ctx.take_modifier_size_hint::<Disabled>();
        let required_size_hint = ctx.take_modifier_size_hint::<Required>();
        let multiple_size_hint = ctx.take_modifier_size_hint::<Multiple>();
        let in_hydration = value.props.in_hydration;
        Pod {
            node: value.node.unchecked_into(),
            props: HtmlInputElement {
                checked: Checked::new(checked_size_hint, in_hydration),
                default_checked: DefaultChecked::new(default_checked_size_hint, in_hydration),
                disabled: Disabled::new(disabled_size_hint, in_hydration),
                required: Required::new(required_size_hint, in_hydration),
                multiple: Multiple::new(multiple_size_hint, in_hydration),
                element_props: value.props,
            },
        }
    }
}

impl<T> With<T> for HtmlInputElement
where
    props::Element: With<T>,
{
    fn modifier(&mut self) -> &mut T {
        self.element_props.modifier()
    }
}

impl With<Checked> for HtmlInputElement {
    fn modifier(&mut self) -> &mut Checked {
        &mut self.checked
    }
}

impl With<DefaultChecked> for HtmlInputElement {
    fn modifier(&mut self) -> &mut DefaultChecked {
        &mut self.default_checked
    }
}

impl With<Disabled> for HtmlInputElement {
    fn modifier(&mut self) -> &mut Disabled {
        &mut self.disabled
    }
}

impl With<Required> for HtmlInputElement {
    fn modifier(&mut self) -> &mut Required {
        &mut self.required
    }
}

impl With<Multiple> for HtmlInputElement {
    fn modifier(&mut self) -> &mut Multiple {
        &mut self.multiple
    }
}
