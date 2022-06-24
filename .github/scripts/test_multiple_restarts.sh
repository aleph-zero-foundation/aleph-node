#!/bin/bash

set -eu

pushd local-tests/

if [ -z "$ALEPH_NODE_BINARY" ]; then
  echo "\$ALEPH_NODE_BINARY must be set in the environment."
  exit 2
fi
if [ ! -f "$ALEPH_NODE_BINARY" ]; then
  echo "Binary $ALEPH_NODE_BINARY does not exist."
  exit 1
fi

echo 'Preparing environment'
chmod +x "$ALEPH_NODE_BINARY"

pip install -r requirements.txt

echo 'Running test'
exec ./test_multiple_restarts.py
