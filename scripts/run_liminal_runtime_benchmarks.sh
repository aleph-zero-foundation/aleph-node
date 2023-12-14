#!/bin/bash

set -euo pipefail

export NODE_ID=5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH

cargo run --release -p aleph-node --features liminal-runtime-benchmarks -- bootstrap-chain \
    --base-path /tmp/ \
    --account-ids $NODE_ID \
    --sudo-account-id $NODE_ID \
    --chain-id a0snarknet \
    --token-symbol SNRKZERO \
    --chain-name 'Aleph Zero Snarknet' \
    > ./chainspec.json

cargo run --release -p aleph-node --features liminal-runtime-benchmarks -- benchmark pallet \
    --chain=chainspec.json \
    --pallet=pallet_vk_storage \
    --extrinsic='*' \
    --steps=20 \
    --repeat=5 \
    --template=.maintain/pallet-weight-template.hbs \
    --wasm-execution=compiled \
    --output=pallets/vk-storage/src/weights.rs
