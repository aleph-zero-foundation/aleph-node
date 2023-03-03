#!/bin/bash

NODE_URL="${NODE_URL:-ws://localhost:9944}"
AUTHORITY="${AUTHORITY:-//Alice}"

function ink-build() {
  docker run \
    --network host \
    -v ${PWD}:/code \
    --platform linux/amd64 \
    --rm public.ecr.aws/p6e8q1z1/ink-dev:0.1.0 "$@"
}

ink-build cargo contract build --release --quiet 1>&2

export ADDER

ADDER=$(
  ink-build cargo contract instantiate --url "$NODE_URL" --suri "$AUTHORITY" --skip-confirm --output-json \
    | jq -r ".contract"
)
echo "$ADDER"
