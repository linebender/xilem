mod generated;
pub use generated::*;

use crate::{
    diff::{diff_kv_iterables, Diff},
    element::{remove_attribute, set_attribute},
    vecmap::VecMap,
    AttributeValue, ChangeFlags, Pod,
};
use xilem_core::Id;

type CowStr = std::borrow::Cow<'static, str>;

// TODO: could be split to struct without generic parameter (to avoid monomorphized bloat (methods below))
/// The state associated with a HTML element `View`.
///
/// Stores handles to the child elements and any child state, as well as attributes and event listeners
pub struct ElementState<ViewSeqState> {
    pub(crate) children_states: ViewSeqState,
    pub(crate) listeners: Vec<(Id, gloo::events::EventListener)>, // used to keep the listeners alive
    pub(crate) id: Id,
    pub(crate) new_attributes: VecMap<CowStr, AttributeValue>,
    pub(crate) attributes: VecMap<CowStr, AttributeValue>,
    pub(crate) child_elements: Vec<Pod>,
    pub(crate) scratch: Vec<Pod>,
}

impl<ViewSeqState> ElementState<ViewSeqState> {
    pub(crate) fn add_new_listener(&mut self, id: Id, value: gloo::events::EventListener) {
        self.listeners.push((id, value));
    }

    pub(crate) fn get_listener(&mut self, id: Id) -> Option<&mut gloo::events::EventListener> {
        self.listeners
            .iter_mut()
            .find(|(listener_id, _)| *listener_id == id)
            .map(|(_, listener)| listener)
    }

    // TODO Not sure how multiple attribute definitions with the same name should be handled (e.g. `e.attr("class", "a").attr("class", "b")`)
    // Currently the outer most (in the example above "b") defines the attribute (when it isn't `None`, in that case the inner attr defines the value)
    pub(crate) fn add_new_attribute(&mut self, name: &CowStr, value: &Option<AttributeValue>) {
        if let Some(value) = value {
            // could be slightly optimized via something like this: `new_attrs.entry(name).or_insert_with(|| value)`
            if !self.new_attributes.contains_key(name) {
                self.new_attributes.insert(name.clone(), value.clone());
            }
        }
    }

    pub(crate) fn apply_attribute_changes(&mut self, element: &web_sys::Element) -> ChangeFlags {
        let mut changed = ChangeFlags::empty();
        // update attributes
        for itm in diff_kv_iterables(&self.attributes, &self.new_attributes) {
            match itm {
                Diff::Add(name, value) | Diff::Change(name, value) => {
                    set_attribute(element, name, &value.serialize());
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
                Diff::Remove(name) => {
                    remove_attribute(element, name);
                    changed |= ChangeFlags::OTHER_CHANGE;
                }
            }
        }
        std::mem::swap(&mut self.attributes, &mut self.new_attributes);
        self.new_attributes.clear();
        changed
    }
}
