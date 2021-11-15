#!/bin/bash

set -e

mkdir -p docker/data/

# source account ids
source docker/env

# Generate chainspec and populate comittee keystores
docker run -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e DAMIAN -e TOMASZ -e ZBYSZKO -e HANSU -e RUST_LOG=info aleph-node:latest -c "aleph-node bootstrap-chain --base-path /data --chain-id a0dnet1 --millisecs-per-block 1000 --session-period 5 --account-ids $DAMIAN,$TOMASZ,$ZBYSZKO,$HANSU --sudo-account-id $DAMIAN > /data/chainspec.json"

# get bootnote peer id
export BOOTNODE_PEER_ID=$(docker run -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e DAMIAN -e RUST_LOG=info aleph-node:latest -c "aleph-node key inspect-node-key --file /data/$DAMIAN/p2p_secret")

echo "BOOTNODE_PEER_ID : $BOOTNODE_PEER_ID"

# Run consensus party
docker-compose -f docker/docker-compose.yml up -d

exit $?
