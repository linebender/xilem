use crate::dom::{attribute::Attr, event::EventListener};

use super::EventTarget;

pub trait Node: EventTarget {
    fn node_name(&self) -> &str;
}

impl<E: Node> Node for Attr<E> {
    fn node_name(&self) -> &str {
        self.element.node_name()
    }
}

impl<E: Node, Ev, F> Node for EventListener<E, Ev, F> {
    fn node_name(&self) -> &str {
        self.element.node_name()
    }
}
