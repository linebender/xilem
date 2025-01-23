// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Async views, allowing concurrent operations, like fetching data from a server

mod task;
pub use task::task;
pub use task::task_raw;
pub use task::ShutdownSignal;
pub use task::Task;
pub use task::TaskProxy;

mod interval;
pub use interval::interval;
pub use interval::Interval;

mod memoized_await;
pub use memoized_await::memoized_await;
pub use memoized_await::MemoizedAwait;
