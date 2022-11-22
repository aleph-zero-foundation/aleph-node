#!/bin/bash

set -e

# built with `docker build --tag aleph-node:snarknet -f ./docker/Dockerfile .`
export NODE_IMAGE=aleph-node:snarknet

# key derived from "//0"
export NODE_ID=5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH
# key derived from "//1"
export HANS=5GBNeWRhZc2jXu7D55rBimKYDk8PGk8itRYFTPfC8RJLKG5o
# Alice well known key
export ALICE=5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

mkdir -p docker/data/

# generate chainspec
docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=debug "${NODE_IMAGE}" -c \
       "aleph-node bootstrap-chain --base-path /data --account-ids $NODE_ID --rich-account-ids $ALICE,$HANS --sudo-account-id $NODE_ID --chain-id a0smnet --token-symbol SNZERO --chain-name 'Aleph Zero Snarknet' > /data/chainspec.snarknet.json"

# Get bootnode peer id
export BOOTNODE_PEER_ID=$(docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=info "${NODE_IMAGE}" -c "aleph-node key inspect-node-key --file /data/$NODE_ID/p2p_secret")

docker-compose -f docker/snarknode-compose.yml up --remove-orphans

exit $?
