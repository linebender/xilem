use std::{fmt::Debug, num::NonZeroU64};

#[derive(Copy, Clone)]
pub struct ViewId {
    routing_id: NonZeroU64,
    debug: &'static str,
}

impl ViewId {
    pub fn for_type<T: 'static>(raw: NonZeroU64) -> Self {
        Self {
            debug: std::any::type_name::<T>(),
            routing_id: raw,
        }
    }

    pub fn routing_id(self) -> NonZeroU64 {
        self.routing_id
    }
}

impl Debug for ViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@[{}]", self.routing_id, self.debug)
    }
}
