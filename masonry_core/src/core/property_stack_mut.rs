use crate::core::{PropertySet, PropertyStack, Selector};

#[derive(Debug)]
pub struct PropertyStackMut<'a> {
    pub(crate) property_stack: &'a mut PropertyStack,
    pub(crate) selector_changes: Vec<Selector>,
}

impl<'a> PropertyStackMut<'a> {
    pub fn edit_selector_property_set(&mut self, selector: Selector) {
        todo!()
    }
    pub fn remove_selector_once(&mut self, selector: &Selector) {
        let maybe_index = self.property_stack.stack.iter().enumerate().rev().find_map(
            |(index, (selector_in, _))| {
                if selector_in == selector {
                    Some(index)
                } else {
                    None
                }
            },
        );
        let Some(index) = maybe_index else {
            return;
        };
        self.property_stack.stack.remove(index);
        self.selector_changes.push(selector.clone());
    }
    pub fn remove_selector_all(&mut self, selector: &Selector) {
        self.property_stack
            .stack
            .retain(|(selector_in, _)| selector_in != selector);
        self.selector_changes.push(selector.clone());
    }
    pub fn push(&mut self, selector: Selector, properties: impl Into<PropertySet>) {
        self.property_stack.push(selector.clone(), properties);
        self.selector_changes.push(selector.clone());
    }
}

#[cfg(test)]
mod tests {
    use peniko::color::palette::css::{BLUE, RED, VIOLET, WHITE, WHITE_SMOKE};

    use crate::{
        core::{ClassSet, Property, PropertyCache},
        layout::Length,
        properties::{Background, BorderWidth},
    };

    use super::*;

    #[test]
    fn test_remove_selector_once() {
        let base_selector = Selector::classes(&["test1"]);
        let base_selector_active = base_selector.clone().with_active(true);
        let mut set = ClassSet::default();
        set.add_class("test1");

        // Initialize property stack
        let mut property_stack = PropertyStack::default();
        {
            property_stack.push(
                base_selector.clone(),
                (
                    Background::Color(VIOLET),
                    BorderWidth::all(Length::px(10.0)),
                ),
            );
            property_stack.push(base_selector_active.clone(), Background::Color(WHITE));
            property_stack.push(base_selector.clone(), Background::Color(WHITE_SMOKE));
            property_stack.push(base_selector.clone(), Background::Color(WHITE_SMOKE));
        }

        // remove tests
        {
            let mut property_stack_mut = PropertyStackMut {
                selector_changes: Vec::new(),
                property_stack: &mut property_stack,
            };
            // Non-active remove test
            {
                property_stack_mut.remove_selector_once(&base_selector);

                assert_eq!(property_stack_mut.property_stack.stack.len(), 3);
                assert_eq!(property_stack_mut.selector_changes.len(), 1);

                assert_eq!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<Background>(&PropertyCache::default(), &set),
                    Some(&Background::Color(WHITE_SMOKE))
                );
            }
            // Active remove test
            {
                let mut set = set.clone();
                set.is_active = true;
                property_stack_mut.remove_selector_once(&base_selector_active);

                assert_eq!(property_stack_mut.property_stack.stack.len(), 2);
                assert_eq!(property_stack_mut.selector_changes.len(), 2);

                assert_eq!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<Background>(&PropertyCache::default(), &set),
                    Some(&Background::Color(WHITE_SMOKE))
                );
            }
        }
    }

    #[test]
    fn test_remove_selector_all() {
        let base_selector = Selector::classes(&["test1"]);
        let base_selector_active = base_selector.clone().with_active(true);
        let base_selector_active_focused =
            base_selector.clone().with_active(true).with_focused(true);
        let mut set = ClassSet::default();
        set.add_class("test1");

        // Initialize property stack
        let mut property_stack = PropertyStack::default();
        {
            property_stack.push(
                base_selector.clone(),
                (
                    Background::Color(VIOLET),
                    BorderWidth::all(Length::px(10.0)),
                ),
            );
            property_stack.push(base_selector_active.clone(), Background::Color(WHITE));
            property_stack.push(base_selector.clone(), Background::Color(WHITE_SMOKE));
            property_stack.push(base_selector.clone(), Background::Color(WHITE_SMOKE));
            property_stack.push(base_selector_active_focused.clone(), Background::Color(RED));
            property_stack.push(base_selector_active.clone(), Background::Color(BLUE));
        }

        // remove tests
        {
            let mut property_stack_mut = PropertyStackMut {
                selector_changes: Vec::new(),
                property_stack: &mut property_stack,
            };
            // Non-active remove test
            {
                property_stack_mut.remove_selector_all(&base_selector);

                assert_eq!(property_stack_mut.property_stack.stack.len(), 3);
                assert_eq!(property_stack_mut.selector_changes.len(), 1);

                assert!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<Background>(&PropertyCache::default(), &set)
                        .is_none(),
                );
            }
            // Active remove test
            {
                let mut set = set.clone();
                set.is_active = true;
                property_stack_mut.remove_selector_all(&base_selector_active);

                assert_eq!(property_stack_mut.property_stack.stack.len(), 1);
                assert_eq!(property_stack_mut.selector_changes.len(), 2);

                assert!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<Background>(&PropertyCache::default(), &set)
                        .is_none(),
                );
            }
            // testing focused active
            {
                let mut set = set.clone();
                set.is_active = true;
                set.has_focus_target = true;
                assert_eq!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<Background>(&PropertyCache::default(), &set),
                    Some(&Background::Color(RED))
                );
            }
        }
    }
}
