#!/bin/bash

set -e

RUST_TOOLCHAIN_VERSION=nightly-2021-10-24

CARGO_INCREMENTAL=0 cargo +$RUST_TOOLCHAIN_VERSION clippy --all-targets --all-features --no-deps
CARGO_INCREMENTAL=0 cargo +$RUST_TOOLCHAIN_VERSION fmt --all
CARGO_INCREMENTAL=0 cargo +$RUST_TOOLCHAIN_VERSION test --lib
