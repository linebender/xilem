use crate::dom::{attribute::Attr, event::EventListener};

use super::EventTarget;

pub trait Node<T, A>: EventTarget<T, A> {
    fn node_name(&self) -> &str;
}

impl<T, A, E: Node<T, A>> Node<T, A> for Attr<E> {
    fn node_name(&self) -> &str {
        self.element.node_name()
    }
}

impl<T, A, E: Node<T, A>, Ev, F> Node<T, A> for EventListener<E, Ev, F> {
    fn node_name(&self) -> &str {
        self.element.node_name()
    }
}
