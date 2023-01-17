{ rustToolchainFile ? ../rust-toolchain }:
rec {
  rustToolchain =
    let
      # use Rust toolchain declared by the rust-toolchain file
      rustToolchain = with nixpkgs; overrideRustTarget ( rustChannelOf { date = "2022-08-12"; channel = "nightly"; } );

      overrideRustTarget = rustChannel: rustChannel // {
        rust = rustChannel.rust.override {
          targets = [ "x86_64-unknown-linux-gnu" "wasm32-unknown-unknown" ];
        };
      };
    in
      rustToolchain;

  nixpkgs =
    let
      # this overlay allows us to use a version of the rust toolchain specified by the rust-toolchain file
      rustOverlay =
        import (builtins.fetchTarball {
          # link: https://github.com/mozilla/nixpkgs-mozilla/tree/f233fdc4ff6ba2ffeb1e3e3cd6d63bb1297d6996
          url = "https://github.com/mozilla/nixpkgs-mozilla/archive/f233fdc4ff6ba2ffeb1e3e3cd6d63bb1297d6996.tar.gz";
          sha256 = "1rzz03h0b38l5sg61rmfvzpbmbd5fn2jsi1ccvq22rb76s1nbh8i";
        });

      # pinned version of nix packages
      # main reason for not using here the newest available version at the time or writing is that this way we depend on glibc version 2.31 (Ubuntu 20.04 LTS)
      nixpkgs = import (builtins.fetchTarball {
        # link: https://github.com/NixOS/nixpkgs/tree/20.09
        url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/20.09.tar.gz";
        sha256 = "1wg61h4gndm3vcprdcg7rc4s1v3jkm5xd7lw8r2f67w502y94gcy";
      }) { overlays = [
             rustOverlay
           ];
         };
    in
      nixpkgs;

  llvm = nixpkgs.llvmPackages_11;

  stdenv = llvm.stdenv;

  # nix helper library for building rust projects
  naersk =
    let
      naerskSrc = builtins.fetchTarball {
        # link: https://github.com/nix-community/naersk/tree/2fc8ce9d3c025d59fee349c1f80be9785049d653
        url = "https://github.com/nix-community/naersk/archive/2fc8ce9d3c025d59fee349c1f80be9785049d653.tar.gz";
        sha256 = "1jhagazh69w7jfbrchhdss54salxc66ap1a1yd7xasc92vr0qsx4";
      };
    in
      nixpkgs.callPackage naerskSrc { inherit stdenv; cargo = rustToolchain.rust; rustc = rustToolchain.rust; };

  # allows to avoid copying into nix-build environment files that are listed by .gitignore
  gitignore =
    let
      gitignoreSrc = builtins.fetchTarball {
        url = "https://github.com/hercules-ci/gitignore.nix/archive/5b9e0ff9d3b551234b4f3eb3983744fa354b17f1.tar.gz";
        sha256 = "o/BdVjNwcB6jOmzZjOH703BesSkkS5O7ej3xhyO8hAY=";
      };
    in
      import gitignoreSrc { inherit (nixpkgs) lib; };

  customRocksDB = import ./rocksdb.nix { inherit nixpkgs; };
}
