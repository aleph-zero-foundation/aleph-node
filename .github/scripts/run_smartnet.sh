#!/bin/bash

set -e

export NODE_IMAGE=public.ecr.aws/p6e8q1z1/aleph-node:latest

# key derived from "//0"
export NODE_ID=5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH
export ALICE=5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

mkdir -p docker/data/

# generate chainspec
docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=debug "${NODE_IMAGE}" -c \
       "aleph-node bootstrap-chain --base-path /data --account-ids $NODE_ID --faucet-account-id $ALICE --sudo-account-id $NODE_ID --chain-id a0smnet --token-symbol SZERO --chain-name 'Aleph Zero Smartnet' > /data/chainspec.smartnet.json"

# Get bootnode peer id
export BOOTNODE_PEER_ID=$(docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=info "${NODE_IMAGE}" -c "aleph-node key inspect-node-key --file /data/$NODE_ID/p2p_secret")

docker network create node-network || true
docker-compose -f docker/smartnet-compose.yml up --remove-orphans
exit $?
