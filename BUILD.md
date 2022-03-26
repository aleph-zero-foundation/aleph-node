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
sudo DOCKER_BUILDKIT=1 docker build -t aleph-node/build -f docker/Dockerfile_build .
sudo docker run -ti --volume=$(pwd):/node/build aleph-node/build
```
Binary will be stored at `$(pwd)/aleph-node`.

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
# spawn nix-shell inside of our docker image
docker run -ti --volume=$(pwd):/node/build aleph-node/build -s
# if your `target` directory contains some artifacts that were not created using this procedure, we first remove them
# otherwise you might receive errors claiming that you are using wrong version of glibc
cargo clean
# build `aleph-node` and store it at the root of the aleph-node's source directory
cargo build --release -p aleph-node
# set the proper loader (nix related)
patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 target/release/aleph-node
```

If you have `nix` installed locally, you can simply call `nix-shell --pure`. It should spawn a shell containing all build
dependencies. Within it, you can call `cargo build --release -p aleph-node`. Keep in mind that a binary created this way will
depend on loader referenced by `nix` and not the default one used by your system. In order to fix it, assuming that your loader
is stored at `/lib64/ld-linux-x86-64.so.2`, you can execute `patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 <path to
aleph-node>`.

## Manual
These are build dependencies we use in our linux images for `aleph-node`:
```
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
sudo apt install build-essential curl git clang libclang-dev pkg-config libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
git clone https://github.com/Cardinal-Cryptography/aleph-node.git
cd aleph-node
rustup show
rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown
cargo build --release
```

[nix]: https://nixos.org/download.html
[rustup]: https://rustup.rs/
[docker]: https://docs.docker.com/engine/install/ubuntu/
[rust-toolchain]: ./rust-toolchain
