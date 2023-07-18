#!/bin/bash

set -e

CARGO_INCREMENTAL=0 cargo clippy --all-targets --all-features -- --no-deps -D warnings
CARGO_INCREMENTAL=0 cargo +nightly-2023-01-10 fmt --all
CARGO_INCREMENTAL=0 cargo test --lib

