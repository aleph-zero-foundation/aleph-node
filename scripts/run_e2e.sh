#!/bin/bash

set -e

cd e2e-tests/

NODE_URL="ws://127.0.0.1:9944" RUST_LOG=info cargo test -- --nocapture --test-threads 1

exit $?
