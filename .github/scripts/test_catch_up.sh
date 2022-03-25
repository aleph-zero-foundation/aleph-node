#!/bin/bash

set -eu

pushd local-tests/

if [ ! -f "$ALEPH_NODE_BINARY" ]; then
  echo "Binary $ALEPH_NODE_BINARY does not exist."
  exit 1
fi

echo 'Preparing environment'
chmod +x "$ALEPH_NODE_BINARY"

pip install -r requirements.txt

echo 'Running test'
exec ./test_catch_up.py
