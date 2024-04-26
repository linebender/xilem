use std::{fmt::Debug, num::NonZeroU64};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ViewId {
    id: NonZeroU64,
    debug: &'static str,
}

impl ViewId {
    pub fn next_with_type<T: 'static>() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        // Note: we can make the safety argument for the unchecked version.
        Self {
            id: (NonZeroU64::new(ID_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap()),
            debug: std::any::type_name::<T>(),
        }
    }

    pub fn id(self) -> NonZeroU64 {
        self.id
    }
}

impl Debug for ViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@[{}]", self.id, self.debug)
    }
}
