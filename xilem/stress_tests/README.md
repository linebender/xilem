# Xilem build stress-tests

The files in this folder are all stress-tests meant to check how long it takes to build a Xilem app in some degenerate cases.

Build the tests with no config flags to quickly check they compile.

To actually make your computer suffer, run this command:

```sh
cargo rustc --profile build-perf --package xilem --test <TEST_NAME> -- --cfg compile_stress_test
```

(Note that, unlike `tests/`, this isn't a folder name recognized by cargo. You need to manually add the tests in this folder to `xilem/Cargo.toml`.)
