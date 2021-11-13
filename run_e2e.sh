#!/bin/bash

set -e

cd e2e-tests/

RUST_LOG=info cargo run -- --node 127.0.0.1:9943

exit $?
