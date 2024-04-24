use std::{fmt::Debug, num::NonZeroU64};

#[derive(Copy, Clone)]
pub struct Id {
    routing_id: NonZeroU64,
    debug: &'static str,
}

impl Id {
    pub fn for_type<T: 'static>(raw: NonZeroU64) -> Self {
        Self {
            debug: std::any::type_name::<T>(),
            routing_id: raw,
        }
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@[{}]", self.routing_id, self.debug)
    }
}
