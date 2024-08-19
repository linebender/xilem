// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Async views, allowing concurrent operations, like fetching data from a server

mod task;
pub use task::{
    task, task_raw, Task, TaskProxy, ShutdownSignal,
};

mod interval;
pub use interval::{interval, Interval};

mod memoized_await;
pub use memoized_await::{memoized_await, MemoizedAwait};
