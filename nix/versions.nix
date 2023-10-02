{ rustToolchainFile ? ../rust-toolchain }:
rec {
  rustToolchain =
    let
      # use Rust toolchain declared by the rust-toolchain.toml file
      rustToolchain = with nixpkgs; overrideRustTarget ( rustChannelOf { channel = "1.70"; } );

      overrideRustTarget = rustChannel: rustChannel // {
        rust = rustChannel.rust.override {
          targets = [ "x86_64-unknown-linux-gnu" "wasm32-unknown-unknown" ];
        };
      };
    in
      rustToolchain;

  nixpkgs =
    let
      # this overlay allows us to use a version of the rust toolchain specified by the rust-toolchain.toml file
      rustOverlay =
        import (builtins.fetchTarball {
          url = "https://github.com/mozilla/nixpkgs-mozilla/archive/7800b921f749d74ecb8456f35f7ef04cd49b4d24.tar.gz";
          sha256 = "1shxjmpmx92q9msh9qy3bz3pk9xcj4rkbphy0q01qgmmrc2f313h";
        });

      # pinned version of nix packages
      # main reason for not using here the newest available version at the time or writing is that this way we depend on glibc version 2.31 (Ubuntu 20.04 LTS)
      nixpkgs = import (builtins.fetchTarball {
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
        url = "https://github.com/nix-community/naersk/archive/d998160d6a076cfe8f9741e56aeec7e267e3e114.tar.gz";
        sha256 = "1s10ygdsi17zjfiypwj7bhxys6yxws10hhq3ckfl3996v2q04d3v";
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
