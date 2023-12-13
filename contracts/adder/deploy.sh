#!/bin/bash

set -euox pipefail

NODE_URL="${NODE_URL:-ws://localhost:9944}"
AUTHORITY="${AUTHORITY:-//Alice}"

function run_ink_builder() {
  docker start ink_builder 1>&2 || docker run \
    --network host \
    -v "${PWD}/../..:/code" \
    -u "$(id -u):$(id -g)" \
    --name ink_builder \
    --platform linux/amd64 \
    --detach \
    --rm public.ecr.aws/p6e8q1z1/ink-dev:1.0.0 sleep 1d 1>&2
}

function ink_build() {
  docker exec \
    -u "$(id -u):$(id -g)" \
    -w "/code/contracts/adder" \
    ink_builder "$@"
}

run_ink_builder
ink_build rustup target add wasm32-unknown-unknown
ink_build rustup component add rust-src
ink_build cargo contract build --release --quiet 1>&2

export ADDER

ADDER=$(
  ink_build cargo contract instantiate --url "$NODE_URL" --suri "$AUTHORITY" --skip-confirm --output-json \
    | jq -r ".contract"
)
echo "$ADDER"
