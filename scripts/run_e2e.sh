#!/bin/bash

set -e

cd e2e-tests/

# TODO till aleph-client would have all the log targets set to aleph-client (and not default aleph_client), we need
# to pass all targets below
RUST_LOG=aleph_e2e_client=info,aleph-client=info,aleph_client=info cargo run -- --node 127.0.0.1:9943

exit $?
