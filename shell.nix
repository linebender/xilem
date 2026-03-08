{ pkgs ? import <nixpkgs> {} }:

pkgs.callPackage (
  {
    mkShell,
    rustc,
    cargo,
    rustPlatform,
    rustfmt,
    clippy,
    rust-analyzer,
  }:
  mkShell rec {
    name = "xilem";
    strictDeps = true;
    nativeBuildInputs = [
      rustc
      cargo
      rustfmt
      clippy
      rust-analyzer
      pkgs.pkg-config
    ];
    buildInputs = with pkgs; [
      fontconfig
      libxkbcommon
      vulkan-loader
      wayland 
    ];

    # Certain Rust tools won't work without this
    RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";
    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
  }
) { }
