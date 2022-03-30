#!/bin/bash

set -eu

pushd local-tests/

if [ ! -f "$OLD_BINARY" ]; then
  echo "Old binary $OLD_BINARY does not exist."
  exit 1
fi

if [ ! -f "$NEW_BINARY" ]; then
  echo "New binary $NEW_BINARY does not exist."
  exit 1
fi

if [ ! -f "$NEW_RUNTIME" ]; then
  echo "New runtime $NEW_RUNTIME does not exist."
  exit 1
fi

echo "Testing runtime update.
      Old binary sha: $(sha256sum $OLD_BINARY)
      New binary sha: $(sha256sum $NEW_BINARY)
      New runtime: $NEW_RUNTIME"

echo 'Preparing environment'
chmod +x $OLD_BINARY $NEW_BINARY

pip install -r requirements.txt

pushd ../bin/cliain/
cargo build --release
popd

echo 'Running test'
./test_update.py

popd
