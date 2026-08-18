use crate::core::{Property, PropertySet};

#[derive(Debug)]
pub struct SelectorPropertiesMut<'a> {
    pub(crate) stack: &'a mut PropertySet,
    pub(crate) has_property_changed: bool,
}

impl<'a> SelectorPropertiesMut<'a> {
    pub fn get<P: Property>(&self) -> Option<&P> {
        self.stack.get()
    }
    pub fn insert<P: Property>(&mut self, value: P) -> Option<P> {
        let old_value = self.stack.insert(value);
        self.has_property_changed = true;
        old_value
    }
    pub fn remove<P: Property>(&mut self) -> Option<P> {
        let old_value = self.stack.remove();
        self.has_property_changed = true;
        old_value
    }
}
