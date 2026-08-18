use crate::core::{Property, PropertySet, PropertyStack, Selector, SelectorPropertiesMut};

/// A rich mutable reference to a [`PropertyStack`].
#[derive(Debug)]
pub struct PropertyStackMut<'a> {
    pub(crate) property_stack: &'a mut PropertyStack,
    pub(crate) selector_changes: Vec<Selector>,
}

impl<'a> PropertyStackMut<'a> {
    /// Edit a property set via its property stack index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn edit_property_set<E, O>(&mut self, index: usize, edit_fn: E) -> O
    where
        E: FnOnce(&mut SelectorPropertiesMut<'_>) -> O,
    {
        let Some((selector, set)) = self.property_stack.stack.get_mut(index) else {
            panic!("The index is out of bound?");
        };
        let mut set_mut = SelectorPropertiesMut {
            has_property_changed: false,
            stack: set,
        };
        let res = edit_fn(&mut set_mut);
        if set_mut.has_property_changed {
            let selector = selector.clone();
            let _ = set;
            self.push_changes(&selector);
        }
        res
    }
    /// Returns the corresponding indexes of the given [`Selector`].
    pub fn get_selector_indexes(&self, selector: &Selector) -> Vec<usize> {
        self.property_stack
            .stack
            .iter()
            .enumerate()
            .filter_map(|(index, (selector_in, _))| {
                if selector == selector_in {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Edit the latest inserted property set for the given [`Selector`].
    ///
    /// Return [`None`] if no property set corresponding the selector is not found.
    pub fn edit_last_selector_property_set<E, O>(
        &mut self,
        selector: &Selector,
        edit_fn: E,
    ) -> Option<O>
    where
        E: FnOnce(&mut SelectorPropertiesMut<'_>) -> O,
    {
        let index = self.get_last_selector_index(selector)?;
        Some(self.edit_property_set(index, edit_fn))
    }
    /// Checks if the given [`Selector`] has any property set present.
    pub fn has_selector(&self, selector: &Selector) -> bool {
        self.property_stack
            .stack
            .iter()
            .any(|(selector_in, _)| selector == selector_in)
    }
    /// Remove the latest inserted property set for the given [`Selector`] (aka `pop`).
    pub fn pop_selector_property_set(&mut self, selector: &Selector) {
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
        self.push_changes(selector);
    }
    /// Remove all property set that is linked to this [`Selector`].
    pub fn remove_selector_all(&mut self, selector: &Selector) {
        self.property_stack
            .stack
            .retain(|(selector_in, _)| selector_in != selector);
        self.push_changes(selector);
    }
    /// Push a new [`PropertySet`] into this stack.
    pub fn push(&mut self, selector: Selector, properties: impl Into<PropertySet>) {
        self.property_stack.push(selector.clone(), properties);
        self.push_changes(&selector);
    }
    /// Remove a property from all property set that is related to this [`Selector`].
    pub fn remove_property<P>(&mut self, selector: &Selector) -> Vec<P>
    where
        P: Property,
    {
        let removed = self
            .property_stack
            .stack
            .iter_mut()
            .flat_map(|(selector_in, set)| {
                if selector_in == selector {
                    set.remove::<P>()
                } else {
                    None
                }
            })
            .collect();
        self.push_changes(selector);
        removed
    }
    /// Shrinks the capacity of the stack and property sets as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.property_stack.stack.iter_mut().for_each(|set| {
            set.1.map.shrink_to_fit();
        });
        self.property_stack.stack.shrink_to_fit();
    }
    /// Remove any stack entry where its property set is empty.
    pub fn remove_empty_sets(&mut self) {
        self.property_stack
            .stack
            .retain(|(_, set)| !set.map.is_empty());
    }
    /// Remove a property stack with its given index.
    pub fn remove_set(&mut self, index: usize) {
        let (selector, set) = self.property_stack.stack.remove(index);
        if !set.map.is_empty() {
            self.push_changes(&selector);
        }
    }
    fn push_changes(&mut self, selector: &Selector) {
        if !self.selector_changes.contains(selector) {
            self.selector_changes.push(selector.clone());
        }
    }
    fn get_last_selector_index(&self, selector: &Selector) -> Option<usize> {
        self.property_stack
            .stack
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, (selector_in, _))| {
                if selector_in == selector {
                    Some(index)
                } else {
                    None
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use peniko::color::palette::css::{BLACK, BLUE, RED, VIOLET, WHITE, WHITE_SMOKE};

    use crate::{
        core::{ClassSet, PropertyCache},
        layout::Length,
        properties::{Background, BorderWidth},
    };

    use super::*;

    #[test]
    fn test_pop_selector() {
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
                property_stack_mut.pop_selector_property_set(&base_selector);

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
                property_stack_mut.pop_selector_property_set(&base_selector_active);

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

    #[test]
    fn test_edit_selector() {
        let base_selector = Selector::classes(&["test1"]);
        // NOTE Not putting the `with_focused(false)` will make the `base_selector_active_focused` properties to shadow-ed by this.
        let base_selector_active = base_selector.clone().with_active(true).with_focused(false);
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
            // property_stack.push(base_selector_active_focused.clone(), Background::Color(RED));
        }

        // edit tests
        {
            let mut property_stack_mut = PropertyStackMut {
                selector_changes: Vec::new(),
                property_stack: &mut property_stack,
            };
            // Non-active edit test
            {
                property_stack_mut
                    .edit_last_selector_property_set(&base_selector, |set| {
                        set.insert(Background::Color(BLUE));
                    })
                    .expect("The selector should be available");
                property_stack_mut.remove_property::<BorderWidth>(&base_selector);

                assert_eq!(property_stack_mut.selector_changes.len(), 1);

                assert_eq!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<Background>(&PropertyCache::default(), &set),
                    Some(&Background::Color(BLUE))
                );

                let border_width = property_stack_mut
                    .property_stack
                    .resolve_without_saving::<BorderWidth>(&PropertyCache::default(), &set);
                // dbg!(border_width);
                assert!(border_width.is_none());
            }
            // Active remove test
            {
                let mut set = set.clone();
                set.is_active = true;
                property_stack_mut
                    .edit_last_selector_property_set(&base_selector_active, |set| {
                        set.insert(Background::Color(BLACK));
                        set.insert(BorderWidth::all(Length::px(10.0)));
                    })
                    .expect("Selector set should be available");

                assert_eq!(property_stack_mut.selector_changes.len(), 2);

                assert_eq!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<Background>(&PropertyCache::default(), &set),
                    Some(&Background::Color(BLACK))
                );
                assert!(
                    property_stack_mut
                        .property_stack
                        .resolve_without_saving::<BorderWidth>(&PropertyCache::default(), &set)
                        .is_some()
                );
            }
            // testing focused active
            {
                let mut set = set.clone();
                set.is_active = true;
                set.has_focus_target = true;
                // BUG normally the end should value should be `RED` instead of `BLACK`?
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
