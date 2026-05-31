# Android examples

All the examples in this folder are just glue code importing from the respective examples in the parent folder.

## Running on Android

The examples are built and packaged with [`cargo-apk`](https://crates.io/crates/cargo-apk).

### Prerequisites

- The Android SDK and NDK, and the `ANDROID_HOME` and `ANDROID_NDK_ROOT` environment variables set so that `cargo-apk` can find them. See the [`cargo-apk` README](https://github.com/rust-mobile/cargo-apk) for the full requirements.
- An Android Rust target installed via `rustup`, for example:

  ```sh
  rustup target add aarch64-linux-android
  ```

- `cargo-apk` itself:

  ```sh
  cargo install cargo-apk --locked
  ```

### Build and run

From the repository root, build any of the examples in this folder with `cargo apk build --example <name>`, for example:

```sh
cargo apk build -p xilem --example calc_android
```

The supported examples are `calc_android`, `emoji_picker_android`, `http_cats_android`, `mason_android`, `stopwatch_android`, `to_do_mvc_android`, and `variable_clock_android`.

To install and run the example on a connected device or running emulator, use `cargo apk run` instead:

```sh
cargo apk run -p xilem --example calc_android
```

`cargo apk check` (with the same target selectors) is used by CI to verify that every Android example still compiles for `aarch64-linux-android`.
