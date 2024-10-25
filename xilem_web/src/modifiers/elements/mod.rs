pub mod html_input_element {
    use crate::overwrite_bool_modifier;
    overwrite_bool_modifier!(Checked);
    overwrite_bool_modifier!(DefaultChecked);
    overwrite_bool_modifier!(Disabled);
    overwrite_bool_modifier!(Required);
    overwrite_bool_modifier!(Multiple);

    pub mod view {
        use crate::modifiers::With;
        use crate::overwrite_bool_modifier_view;
        overwrite_bool_modifier_view!(Checked);
        overwrite_bool_modifier_view!(DefaultChecked);
        overwrite_bool_modifier_view!(Disabled);
        overwrite_bool_modifier_view!(Required);
        overwrite_bool_modifier_view!(Multiple);
    }
}
