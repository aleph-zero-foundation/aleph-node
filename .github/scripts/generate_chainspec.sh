#!/bin/bash

set -e

# populate validators keystore and generate chainspec
chmod +x target/release/aleph-node
./target/release/aleph-node bootstrap-chain --base-path docker/data --chain-id a0dnet1 --n-members 4 --session-period 5 --millisecs-per-block 1000 > docker/data/chainspec.json

echo "Done"
exit $?
