#!/bin/bash

NODE_URL="${NODE_URL:-ws://localhost:9944}"
AUTHORITY="${AUTHORITY:-//Alice}"

cargo contract build --release --quiet 1>&2
cargo contract upload --url "$NODE_URL" --suri "$AUTHORITY" --quiet 1>&2

export ADDER

ADDER=$(
  cargo contract instantiate --url "$NODE_URL" --suri "$AUTHORITY" --skip-confirm --output-json \
    | jq -r ".contract"
)
echo "$ADDER"
