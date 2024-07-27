// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Async views, allowing concurrent operations, like fetching data from a server

mod await_once;
pub use await_once::{await_once, AwaitOnce};

mod memoized_await;
pub use memoized_await::{memoized_await, MemoizedAwait};

