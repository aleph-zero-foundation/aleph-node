#!/bin/bash

set -e

mkdir -p docker/data/

# source account ids
source docker/env

# Generate chainspec and populate comittee keystores
# hidden assumption: --chain-id must be present and different from a0dnet1, otherwise default chainspec
# would generate some non-empty validators, and we want to start from the scratch
docker run -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e DAMIAN -e TOMASZ -e ZBYSZKO -e HANSU -e JULIA -e RUST_LOG=info \
 aleph-node:latest -c "aleph-node bootstrap-chain --base-path /data --chain-id a0dnet1-e2e --account-ids $DAMIAN,$TOMASZ,$ZBYSZKO,$HANSU,$JULIA --sudo-account-id $DAMIAN > /data/chainspec.json"

# get bootnote peer id
export BOOTNODE_PEER_ID=$(docker run -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e DAMIAN -e RUST_LOG=info aleph-node:latest -c "aleph-node key inspect-node-key --file /data/$DAMIAN/p2p_secret")

echo "BOOTNODE_PEER_ID : $BOOTNODE_PEER_ID"

# Run consensus party
docker-compose -f docker/docker-compose.yml up -d

exit $?
