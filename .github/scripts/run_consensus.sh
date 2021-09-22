#!/bin/bash

set -e

mkdir -p docker/data/

# populate validators keystore and generate chainspec
chmod +x target/release/aleph-node

# Generate chainspec and populate comittee keystores
./target/release/aleph-node bootstrap-chain --base-path docker/data --chain-id a0dnet1 --n-members 4 --session-period 5 --millisecs-per-block 1000 > docker/data/chainspec.json

# get bootnote peer id
export BOOTNODE_PEER_ID=$(./target/release/aleph-node key inspect-node-key --file docker/data/5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH/p2p_secret)

echo "BOOTNODE_PEER_ID : $BOOTNODE_PEER_ID"

# Run consensus party
docker-compose -f docker/docker-compose.yml up -d

exit $?
