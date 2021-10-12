use std::any::Any;
use std::collections::VecDeque;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct PromiseTokenId(NonZeroU64);

pub struct PromiseToken<T = ()>(PromiseTokenId, std::marker::PhantomData<T>);

#[derive(Clone, Debug)]
pub struct PromiseResult {
    token_id: PromiseTokenId,
    // TODO - Rework command system to remove Mutex
    payload: Arc<Mutex<Option<Box<dyn Any + Send>>>>,
}

// ---

impl PromiseTokenId {
    pub fn next() -> PromiseTokenId {
        use druid_shell::Counter;
        static WIDGET_ID_COUNTER: Counter = Counter::new();
        PromiseTokenId(WIDGET_ID_COUNTER.next_nonzero())
    }

    pub fn to_raw(self) -> u64 {
        self.0.into()
    }
}

impl<T: Any + Send> PromiseToken<T> {
    pub fn new() -> PromiseToken<T> {
        let promise_token_id = PromiseTokenId::next();
        PromiseToken(promise_token_id, std::marker::PhantomData)
    }

    // TODO - remove; make it possible to get token when constructin widget.
    /// Returns a promise that will never be resolved.
    pub fn empty() -> PromiseToken<T> {
        Self::new()
    }

    pub(crate) fn id(&self) -> PromiseTokenId {
        self.0
    }

    pub fn make_result(&self, payload: T) -> PromiseResult {
        PromiseResult {
            token_id: self.0,
            payload: Arc::new(Mutex::new(Some(Box::new(payload)))),
        }
    }
}

impl PromiseResult {
    pub(crate) fn token_id(&self) -> PromiseTokenId {
        self.token_id
    }

    pub(crate) fn get_payload(&self) -> Box<dyn Any + Send> {
        self.payload
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| panic!("Cannot resolve promise: payload already taken."))
    }
}

impl PromiseResult {
    pub fn is<T: Any + Send>(&self, token: PromiseToken<T>) -> bool {
        self.token_id == token.0
    }

    pub fn try_get<T: Any + Send>(&self, token: PromiseToken<T>) -> Option<T> {
        if self.token_id == token.0 {
            let payload = self.get_payload();
            let payload = payload.downcast::<T>().unwrap_or_else(|_| {
                // This one should never happen given the public API given to users.
                panic!("Cannot resolve promise: wrong payload type.")
            });
            Some(*payload)
        } else {
            None
        }
    }

    pub fn get<T: Any + Send>(&self, token: PromiseToken<T>) -> T {
        self.try_get(token)
            .unwrap_or_else(|| panic!("Cannot resolve promise: mismatched token."))
    }
}

// ---

impl<T> Copy for PromiseToken<T> {}

impl<T> Clone for PromiseToken<T> {
    fn clone(&self) -> Self {
        PromiseToken(self.0, std::marker::PhantomData)
    }
}

impl<T> std::fmt::Debug for PromiseToken<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("PromiseToken")
            .field(&self.0.to_raw())
            .finish()
    }
}
