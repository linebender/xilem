# How to contribute

Issues and PRs are welcome. See [help-wanted issues](https://github.com/PoignardAzur/masonry-rs/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) if you don't know where to begin.

## Issues

Issues don't have a specific format.

If you submit one, please be polite, remember that most Masonry contributors are unpaid volunteers, and include detailed steps to reproduce any problem.


## Pull requests

All submissions, including submissions by project members, require review. We use GitHub Pull Requests for this purpose. Consult GitHub Help for more information on using pull requests.

Before making a PR, please follow these steps:

- Run `cargo test --all-targets` on your code.
- Run `cargo test --doc`.
- Run `cargo clippy --all-targets`.

Masonry doesn't keep a changelog for now, but might in the future. Make sure your commit messages and pull request titles explain what you have changed.
How to maintain


## Preparing for a new release

Before a new release, make sure to update every dependency to their last version.

You can use `cargo upgrade` from the `cargo-edit` tool to help with the process.
