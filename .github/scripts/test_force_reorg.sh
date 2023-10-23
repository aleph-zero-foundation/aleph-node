#!/usr/bin/env bash

set -eu

pushd local-tests/
ALEPH_NODE_BINARY=${ALEPH_NODE_BINARY:-'../target/release/aleph-node'}

if [ ! -f "$ALEPH_NODE_BINARY" ]; then
  echo "Binary $ALEPH_NODE_BINARY does not exist."
  exit 1
fi

echo 'Preparing environment'
chmod +x "$ALEPH_NODE_BINARY"

pip install -r requirements.txt

echo 'Running test'
export PYTHONUNBUFFERED=y
exec ./test_force_reorg.py
