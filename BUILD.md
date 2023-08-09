# Building

## Table of Contents
1. [Build with Docker](#Build-with-Docker) (choose this if not sure)
2. [Build with Nix](#Build-with-Nix)
3. [Manual](#Manual)

## Build with Docker

### Requirements
1. [docker][docker]

In order to build a binary for `aleph-node` using docker we first need to install docker itself, i.e. in case of the Ubuntu Linux
distribution, by executing `sudo apt install docker.io` (please consult your distribution's manual describing docker
installation procedure). Build procedure can be invoked by running:
```
sudo docker build -t aleph-build -f nix/Dockerfile.build .
sudo docker run -ti --volume=$(pwd):/node/build aleph-build
```
Binary will be stored at `$(pwd)/result/bin/aleph-node`.
In order to build just the `aleph-runtime`, execute:
```
sudo docker run -ti --volume=$(pwd):/node/build --env CRATES='{ "aleph-runtime" = []; }' aleph-build`.
```

## Build with Nix

### Requirements
1. [nix][nix]
2. glibc in version ≥ 2.31

The docker approach described above is based on the `nix` package manager.
We can spawn a shell instance within that docker container that includes references to all build dependencies of `aleph-node`.
Within it we can call `cargo build`.
This way, our docker instance maintains all build artifacts inside of project's root directory, which allows to speed up
ongoing build invocations, i.e. next time one invokes `cargo build` it should take significantly less time.
```
# create the builder image
sudo docker build -t aleph-build -f nix/Dockerfile.build .
# spawn nix-shell inside of our docker image
sudo docker run -ti --volume=$(pwd):/node/build --entrypoint="nix-shell" aleph-build --pure
# if your `target` directory contains some artifacts that were not created using this procedure, we first remove them
# otherwise you might receive errors claiming that you are using wrong version of glibc
cargo clean
# build `aleph-node` and store it at the root of the aleph-node's source directory
cargo build --release --package aleph-node
# set the proper loader (nix related)
patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 --set-rpath /lib/x86_64-linux-gnu/ target/release/aleph-node
```

If you have `nix` installed locally, you can simply call `nix-shell --pure`. It should spawn a shell containing all build
dependencies. Within it, you can call `cargo build --release -p aleph-node`. Keep in mind that a binary created this way will
depend on loader referenced by `nix` and not the default one used by your system. In order to fix it, assuming that your loader
is stored at `/lib64/ld-linux-x86-64.so.2`, you can execute `patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 <path to
aleph-node>`. Alternatively, you can use our nix-build script (used by docker based approach), i.q. `nix/nix-build.sh`.

Note: we recommend using `direnv` together with `nix-direnv` for setting up nix-shell. This way you can use your preferred shell,
instead of one provided by nix-shell.
Example configuration for `direnv`. Copy it into `.envrc` file and then run `direnv-allow`:
```
# installs nix-direnv, https://github.com/nix-community/nix-direnv
if ! has nix_direnv_version || ! nix_direnv_version 2.1.0; then
  source_url "https://raw.githubusercontent.com/nix-community/nix-direnv/2.1.0/direnvrc" "sha256-FAT2R9yYvVg516v3LiogjIc8YfsbWbMM/itqWsm5xTA="
fi
use nix
```

## Manual
These are build dependencies we use in our linux images for `aleph-node`:
```
rust-nightly-2021-10-24
bash-4.4
glibc-2.31
binutils-2.36,1
clang-11.0.0rc2
protobuf-3.13.0
openssl-1.1.1g
git-2.28.0
nss-cacert-3.56
pkg-config-0.29.2
rocksdb-6.29.3
```

Version of the rust toolchain is specified by the [rust-toolchain][rust-toolchain] file within this repository. You can use [rustup][rustup] to install a specific
version of rust, including its custom compilation targets. Using `rustup`, it should set a proper toolchain automatically while
you call `rustup show` within project's root directory. Naturally, we can try to use different versions of these dependencies,
i.e. delivered by system's default package manager (we provide a short guide below). Notice, that the `nix` based approach
is not referencing any of the `gcc` compiler tools, where for example ubuntu's package `build-essential` already includes `gcc`.
It might influence some of the build scripts of our build dependencies and it might be necessary to carefully craft some of
the environment flags related with the build process, like `CXXFLAGS` etc.
Example build procedure using Ubuntu 20.04 LTS and bash shell:
```
sudo apt install build-essential curl git clang libclang-dev pkg-config libssl-dev protobuf-compiler
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
git clone https://github.com/Cardinal-Cryptography/aleph-node.git
cd aleph-node
rustup show
rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown
cargo build --release
```

If `cargo build --release` does not succeed but throws an error mentioning `Rust WASM toolchain not installed, please install it!`, then please issue the `rustup target add wasm32-unknown-unknown` command **inside of the aleph-node** directory.

After a successful build the binary can be found in `target/release/aleph-node`.


[nix]: https://nixos.org/download.html
[rustup]: https://rustup.rs/
[docker]: https://docs.docker.com/engine/install/ubuntu/
[rust-toolchain]: ./rust-toolchain.toml
