#!/bin/bash

if [[ -z "$CHAIN_NAME" ]]; then
  echo "You must provide CHAIN_NAME environment variable" 1>&2
  exit 1
fi

if [[ -z "$BASE_PATH" ]]; then
  echo "You must provide BASE_PATH environment variable" 1>&2
  exit 1
fi

if [[ -z "$NODE_NAME" ]]; then
  echo "You must provide NODE_NAME environment variable" 1>&2
  exit 1
fi

if [[ -z "$NODE_KEY_PATH" ]]; then
  echo "You must provide NODE_KEY_PATH environment variable" 1>&2
  exit 1
fi

if [[ -z "$RESERVED_NODES" ]]; then
  echo "You must provide RESERVED_NODES environment variable" 1>&2
  exit 1
fi

./aleph-node --validator \
 --chain $CHAIN_NAME \
 --base-path $BASE_PATH \
 --name $NODE_NAME \
 --node-key-file $NODE_KEY_PATH \
 --rpc-port 9933 \
 --ws-port 9944 \
 --port 30334 \
 --rpc-cors all \
 --rpc-methods Safe \
 --execution Native \
 --no-prometheus \
 --no-telemetry \
 --reserved-only \
 --reserved-nodes $RESERVED_NODES
