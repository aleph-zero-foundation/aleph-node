#!/bin/bash

set -e

# source docker/env

docker run -v $(pwd)/docker/data:/data --network container:damian -e NODE_URL=127.0.0.1:9943 -e RUST_LOG=info aleph-e2e-client:latest

exit $?
