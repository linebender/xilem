// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The boilerplate run function for desktop platforms

use xilem::{EventLoop, winit::error::EventLoopError};

fn main() -> Result<(), EventLoopError> {
    placehero::run(EventLoop::with_user_event())
}
