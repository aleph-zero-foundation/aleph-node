#!/bin/bash

set -e

CARGO_INCREMENTAL=0 cargo +nightly-2021-07-21 clippy --all-targets --all-features
CARGO_INCREMENTAL=0 cargo +nightly-2021-07-21 fmt --all
CARGO_INCREMENTAL=0 cargo test --lib
