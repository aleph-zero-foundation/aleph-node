#!/bin/bash

set -e

CARGO_INCREMENTAL=0 cargo clippy --all-targets --all-features -- --no-deps -D warnings
CARGO_INCREMENTAL=0 cargo fmt --all
CARGO_INCREMENTAL=0 cargo test --lib
