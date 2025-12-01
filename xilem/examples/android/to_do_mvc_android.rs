// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A to-do-list app, loosely inspired by todomvc.

#[cfg(target_os = "android")]
#[path = "../to_do_mvc.rs"]
mod to_do_mvc;

#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use to_do_mvc::run;
    use winit::platform::android::EventLoopBuilderExtAndroid;
    use xilem::EventLoop;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
