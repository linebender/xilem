use std::fmt::Debug;

#[derive(Copy, Clone)]
pub struct ViewId {
    // TODO: This used to be NonZeroU64, but that wasn't really being used
    routing_id: u64,
    debug: &'static str,
}

impl ViewId {
    pub fn for_type<T: 'static>(raw: u64) -> Self {
        Self {
            debug: std::any::type_name::<T>(),
            routing_id: raw,
        }
    }

    pub fn routing_id(self) -> u64 {
        self.routing_id
    }
}

impl Debug for ViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@[{}]", self.routing_id, self.debug)
    }
}
