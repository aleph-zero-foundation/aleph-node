#!/bin/bash

set -e

cd e2e-tests/

RUST_LOG=aleph_e2e_client=info,aleph-client=info cargo run -- --node 127.0.0.1:9943

exit $?
