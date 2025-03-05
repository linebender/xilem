// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Async views, allowing concurrent operations, like fetching data from a server

mod task;
pub use task::{ShutdownSignal, Task, TaskProxy, task, task_raw};

mod interval;
pub use interval::{Interval, interval};

mod memoized_await;
pub use memoized_await::{MemoizedAwait, memoized_await};
