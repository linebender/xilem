// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example demonstrating the use of Async web requests in Xilem to access the <https://http.cat/> API.
//! This also demonstrates image loading.

mod http_cats;

#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use http_cats::run;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
