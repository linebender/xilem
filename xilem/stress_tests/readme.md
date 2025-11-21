# Xilem build stress-tests

The files in this folder are all stress-tests meant to check how long it takes to build a Xilem app in some degenerate cases.

Build the tests with no feature flags to quickly check they compile.

Build the tests with `--features compile-stress-test` to actually make your computer suffer.

(Note that, unlike `tests/`, this isn't a folder name recongized by cargo. You need to manually add the tests in this folder to `xilem/Cargo.toml`.)
