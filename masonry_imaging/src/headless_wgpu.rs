// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

/// Errors that can occur while creating a headless WGPU device.
#[derive(Debug)]
pub(crate) enum InitError {
    NoAdapter,
    RequestDevice(wgpu::RequestDeviceError),
}

impl core::fmt::Display for InitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoAdapter => write!(f, "no compatible WGPU adapter found"),
            Self::RequestDevice(err) => write!(f, "requesting WGPU device failed: {err}"),
        }
    }
}

impl std::error::Error for InitError {}

pub(crate) fn try_init_device_and_queue() -> Result<(wgpu::Device, wgpu::Queue), InitError> {
    block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|_| InitError::NoAdapter)?;

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("masonry_imaging headless device"),
                required_features: wgpu::Features::empty(),
                ..Default::default()
            })
            .await
            .map_err(InitError::RequestDevice)
    })
}

fn block_on<F: Future>(future: F) -> F::Output {
    struct ThreadWaker(std::thread::Thread);

    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}
