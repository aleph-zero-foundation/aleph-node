{ buildOptions ? {}
, rustToolchainFile ? ./rust-toolchain
}:
let
  versions = import ./nix/versions.nix { inherit rustToolchainFile; };
  nixpkgs = versions.nixpkgs;
  env = versions.stdenv;
  project = import ./default.nix ( buildOptions // { inherit versions; } );
  rust = nixpkgs.rust.override {
    extensions = [ "rust-src" ];
  };
  nativeBuildInputs = [rust nixpkgs.cacert nixpkgs.openssl] ++ project.nativeBuildInputs;
in
nixpkgs.mkShell.override { stdenv = env; }
  {
    inherit nativeBuildInputs;
    inherit (project) buildInputs shellHook;
  }
