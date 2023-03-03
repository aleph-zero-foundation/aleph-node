{ buildOptions ? {}
, rustToolchainFile ? ./rust-toolchain
}:
let
  versions = import ./nix/versions.nix { inherit rustToolchainFile; };
  nixpkgs = versions.nixpkgs;
  env = versions.stdenv;
  project = import ./default.nix ( buildOptions // { inherit versions; } );
  rust = versions.rustToolchain.rust.override {
    extensions = [ "rust-src" ];
  };
  nativeBuildInputs = [rust nixpkgs.cacert nixpkgs.openssl] ++ project.nativeBuildInputs;
in
nixpkgs.mkShell.override { stdenv = env; }
  {
    inherit nativeBuildInputs;
    inherit (project) buildInputs shellHook;
    # RUST_SRC_PATH might be needed by the `rust-analyzer`
    RUST_SRC_PATH = "${versions.rustToolchain.rust-src}/lib/rustlib/src/rust/library/";
  }
