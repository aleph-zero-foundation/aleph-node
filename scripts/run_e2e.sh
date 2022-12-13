#!/bin/bash

set -e

cd e2e-tests/

E2E_CONFIG="--node ws://127.0.0.1:9943" RUST_LOG=info cargo test -- --nocapture

exit $?
