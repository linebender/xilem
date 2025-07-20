{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:nixos/nixpkgs";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };
        msrvRustVersion = (pkgs.lib.importTOML ../Cargo.toml).workspace.package.rust-version;
        # The rust-version in Cargo.toml is usually something like "1.88" but we need the full semantic version "1.88.0"
        rustOverlayVersion = if builtins.length (builtins.split "\\." msrvRustVersion) == 3 then "${msrvRustVersion}.0" else msrvRustVersion;
        rustToolchain = pkgs.rust-bin.stable."${rustOverlayVersion}".default.override {
          extensions = [ "rustfmt" "rust-analyzer" "rust-src" ];
          targets = [ "x86_64-unknown-linux-gnu" "wasm32-unknown-unknown" "x86_64-pc-windows-gnu" "aarch64-linux-android" ];
        };

        commonPackages = [
          # comment this out if you want to use your system rust toolchain
          rustToolchain
        ];
        masonryPackages = with pkgs; commonPackages ++ [
          pkg-config

          fontconfig

          libxkbcommon
          xorg.libxcb
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          xorg.libXxf86vm

          vulkan-loader

          wayland
          wayland-protocols
          wayland-scanner
        ];

        webPackages = commonPackages ++ [ pkgs.trunk ];

        mkDevShell = packages: pkgs.mkShell {
          inherit packages;
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath packages}";
        };
      in
      {
        devShells.default = mkDevShell (masonryPackages ++ webPackages);

        devShells.xilem_web = mkDevShell webPackages;
        devShells.xilem = mkDevShell masonryPackages;
        devShells.masonry = mkDevShell masonryPackages;
      });
}
