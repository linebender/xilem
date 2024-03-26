// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

#![allow(missing_docs)]

use std::any::Any;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct PromiseTokenId(NonZeroU64);

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
    #[allow(clippy::new_without_default)]
    pub fn new() -> PromiseToken<T> {
        let promise_token_id = PromiseTokenId::next();
        PromiseToken(promise_token_id, std::marker::PhantomData)
    }

    // TODO - remove; make it possible to get token when constructing widget.
    /// Returns a promise that will never be resolved.
    pub fn empty() -> PromiseToken<T> {
        Self::new()
    }

    pub fn make_result(&self, payload: T) -> PromiseResult {
        PromiseResult {
            token_id: self.0,
            payload: Arc::new(Mutex::new(Some(Box::new(payload)))),
        }
    }
}

impl PromiseResult {
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

#[allow(clippy::non_canonical_clone_impl)]
#[cfg(not(tarpaulin_include))]
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

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_empty_token() {
        let promise_token: PromiseToken<i32> = PromiseToken::empty();
        dbg!(promise_token);
    }

    #[test]
    fn create_and_use_promise() {
        let promise_token = PromiseToken::new();
        let promise_result = promise_token.make_result(42);
        assert!(promise_result.is(promise_token));
        assert_eq!(promise_result.get(promise_token), 42);
    }

    #[test]
    fn bad_promise() {
        let promise_token_1: PromiseToken<i32> = PromiseToken::new();
        let promise_token_2: PromiseToken<i32> = PromiseToken::new();

        let promise_result = promise_token_1.make_result(42);
        assert!(!promise_result.is(promise_token_2));
        assert!(promise_result.try_get(promise_token_2).is_none());
    }

    #[should_panic]
    #[test]
    fn bad_promise_get() {
        let promise_token_1: PromiseToken<i32> = PromiseToken::new();
        let promise_token_2: PromiseToken<i32> = PromiseToken::new();

        let promise_result = promise_token_1.make_result(42);
        promise_result.get(promise_token_2);
    }

    #[should_panic]
    #[test]
    fn get_promise_twice() {
        let promise_token = PromiseToken::new();
        let promise_result = promise_token.make_result(42);

        promise_result.get(promise_token);
        promise_result.get(promise_token);
    }
}
