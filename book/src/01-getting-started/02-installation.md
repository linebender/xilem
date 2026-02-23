# Installation and Setup

Before you can start building with Xilem, you need to get your development environment ready. This involves making sure you have Rust itself installed, along with some platform-specific dependencies that Xilem's underlying libraries need to talk to your operating system's graphics and windowing systems.

The good news is that once you've done this setup once, you won't need to think about it again. The better news is that on most platforms, what you need is straightforward to install using your system's package manager.

## Installing Rust

If you don't already have Rust installed, head to [rustup.rs](https://rustup.rs/) and follow the instructions there. Rustup is the official Rust installer and version manager, and it's the easiest way to get a working Rust toolchain on any platform.

After running the rustup installer, you can verify everything worked by opening a new terminal and running:
```bash
rustc --version
```

You should see output showing the Rust compiler version. **Xilem requires Rust 1.88 or later**, so make sure your version is at least that recent. If you already had Rust installed but it's older, you can update with:
```bash
rustup update
```

> **Note on Rust versions:** Xilem is experimental software under active development, and future versions may increase the minimum Rust version requirement. The Xilem team doesn't treat this as a breaking change, so even minor releases might bump the MSRV. If you encounter compiler errors about features not being available, first try updating your Rust toolchain.

## Platform-Specific Dependencies

Xilem builds on top of several libraries that interact with your operating system to create windows, render graphics, and handle input. These libraries need some system packages to be installed. The requirements are documented for Linux and BSD systems.

### Linux and BSD

Linux and BSD systems require development headers for several libraries that Xilem uses. Most distributions come with `pkg-config` installed by default, but you'll need to add development packages for `clang`, `wayland`, `libxkbcommon`, `libxcb`, and `vulkan-loader`.

**On Fedora**, run:
```bash
sudo dnf install clang wayland-devel libxkbcommon-x11-devel libxcb-devel vulkan-loader-devel
```

**On Debian or Ubuntu**, run:
```bash
sudo apt-get install clang libwayland-dev libxkbcommon-x11-dev libvulkan-dev
```

These packages give Rust's build system access to the headers and libraries it needs to compile code that talks to the display server, handles keyboard input properly, and uses Vulkan for GPU-accelerated graphics.

**For other Linux distributions or BSD variants**, look for development packages with similar names in your package manager. The exact package names might vary, but you're looking for the same underlying libraries: clang, wayland, libxkbcommon, libxcb, and vulkan-loader (or vulkan-icd-loader on some systems).

### NixOS

If you're using NixOS, the Xilem repository includes a development flake that sets up the environment with all necessary dependencies. From the root of the Xilem repository, you can run:
```bash
# For working with all crates
nix develop ./docs

# For working with specific crates
nix develop ./docs#xilem
nix develop ./docs#masonry
nix develop ./docs#xilem_web
```

> **Important:** This flake is provided as a starting point and is not routinely validated by the core team. The team does not require contributors to ensure that this accurately reflects the build requirements, as most contributors and maintainers are not using NixOS. If it is out of date, please open an issue or PR to help keep it current.

### Other Platforms

For platforms not specifically documented here, the general requirement is that you need a working Rust toolchain and any system libraries that the underlying graphics and windowing libraries depend on. If you run into issues during compilation, the error messages will typically indicate which system libraries are missing. The Xilem community in the Zulip chat can help troubleshoot platform-specific setup problems.

## Verifying Your Installation

The best way to verify everything is working is to actually build and run a Xilem example. The Xilem repository includes several example applications you can use to test your setup.

First, clone the Xilem repository:
```bash
git clone https://github.com/linebender/xilem.git
cd xilem
```

Then try running one of the examples, like the to-do list application:
```bash
cargo run --example to_do_mvc
```

The first time you run this, Cargo will download and compile all of Xilem's dependencies. This might take several minutes depending on your machine's speed and internet connection. **This is normal** - Rust compiles everything from source, and Xilem pulls in quite a few libraries. Subsequent builds will be much faster because Cargo caches compiled dependencies.

If everything is set up correctly, you should see a window appear with a working to-do list application. You can add tasks, mark them as complete, and see the UI update reactively as you interact with it. Try clicking around to verify that the application is responsive and rendering properly.

> **If the build fails**, read the error messages carefully. Compiler errors about missing libraries usually mean you're missing a system dependency mentioned in the platform-specific sections above. Errors about Rust language features not being available mean your Rust version is too old - run `rustup update` to get the latest.

## Setting Up a New Project

Once you've verified that Xilem works on your system, you're ready to start your own project. Create a new Rust project with Cargo:
```bash
cargo new my-xilem-app
cd my-xilem-app
```

Then add Xilem as a dependency:
```bash
cargo add xilem
```

This adds the latest version of Xilem to your `Cargo.toml`. You can verify it's there by opening `Cargo.toml` and looking for Xilem in the `[dependencies]` section.

Your project is now set up and ready for you to start writing Xilem code. The next chapter will walk you through creating your first application from scratch, explaining each piece as we build it.

## Optional: Recommended Cargo Configuration

If you're planning to contribute to Xilem itself or work extensively with the Xilem repository rather than just using Xilem in your own projects, there's one optimization that will save you a lot of disk space.

The Xilem repository contains many crates and examples, which means building it compiles a lot of code and creates a large `target/` directory. On Linux and macOS, you can reduce the size of this directory significantly by using split debuginfo. This tells the compiler to write one debuginfo file per dependency instead of bundling everything together.

Create or edit `.cargo/config.toml` in your home directory or in the Xilem repository root and add:
```toml
[profile.dev]
# One debuginfo file per dependency, to reduce file size of tests/examples.
# Note that this value is not supported on Windows.
# See https://doc.rust-lang.org/cargo/reference/profiles.html#split-debuginfo
split-debuginfo="unpacked"
```

This is particularly helpful if you're working on Xilem examples or running tests frequently, as it can cut the `target/` directory size roughly in half. The trade-off is negligible—debugging still works normally, builds aren't noticeably slower, and the only real difference is how the debug information is organized on disk.

> **Windows users:** This setting is not supported on Windows, so don't add it if you're developing on Windows. The Rust compiler will complain if you try.

---

With your environment set up and verified, you're ready to write your first Xilem application. The next chapter walks through a complete working example, explaining how each part fits together and introducing you to the patterns you'll use in every Xilem application you build.