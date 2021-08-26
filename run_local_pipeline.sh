#!/bin/bash

set -e

CARGO_INCREMENTAL=0 cargo +nightly clippy --all-targets --all-features
CARGO_INCREMENTAL=0 cargo +nightly fmt --all
CARGO_INCREMENTAL=0 cargo test --lib
