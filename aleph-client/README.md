# API for [aleph-node](https://github.com/Cardinal-Cryptography/aleph-node) chain.

This crate provides a Rust application interface for submitting transactions to `aleph-node` chain.
Most of the [pallets](https://docs.substrate.io/reference/frame-pallets/) are common to any
[Substrate](https://github.com/paritytech/substrate) chain, but there are some unique to `aleph-node`,
e.g. [`pallets::elections::ElectionsApi`](./src/pallets/elections.rs).

## Build

Just use `cargo build` or `cargo build --release`, depends on your usecase.

## Contributions

All contributions are welcome, e.g. adding new API for pallets in `aleph-node`. 

## Metadata

`aleph-client` uses [`subxt`](https://github.com/paritytech/subxt) to communicate with a Substrate-based chain which
`aleph-node` is. In order to provide a strong type safety, it uses a manually generated file [`aleph_zero.rs`](src/aleph_zero.rs)
which refers to top of the `main` branch in `aleph-node` repository. See more info [here](docker/README.md).
