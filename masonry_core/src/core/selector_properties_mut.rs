use crate::core::{Property, PropertySet};

/// A mutable reference to a [`PropertySet`].
#[derive(Debug)]
pub struct SelectorPropertiesMut<'a> {
    pub(crate) stack: &'a mut PropertySet,
    pub(crate) has_property_changed: bool,
}

impl<'a> SelectorPropertiesMut<'a> {
    /// Returns value of property `P`.
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.stack.get()
    }
    /// Sets property `P` to given value. Returns the previous value if `P` was already set.
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        let old_value = self.stack.insert(value);
        self.has_property_changed = true;
        old_value
    }
    /// Removes property `P`. Returns the previous value if `P` was set.
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        let old_value = self.stack.remove();
        self.has_property_changed = true;
        old_value
    }
}
