#!/bin/bash
set -euo pipefail
echo "Starting DB sync test."

BOOT_NODES=/dns4/bootnode-eu-central-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L/dns4/bootnode-eu-west-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k/dns4/bootnode-eu-west-2-0.test.azero.dev/tcp/30333/p2p/12D3KooWAkqYFFKMEJn6fnPjYnbuBBsBZq6fRFJZYR6rxnuCZWCC/dns4/bootnode-us-east-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWQFkkFr5aM5anGEiUCQiGUdRyWgrdpvSjBgWAUS9srLE4/dns4/bootnode-us-east-2-0.test.azero.dev/tcp/30333/p2p/12D3KooWD5s2dkifJua69RbLwEREDdJjsNHvavNRGxdCvzhoeaLc
BASE_PATH="running/"
CHAINSPEC="${BASE_PATH}/chainspec.json"
DB_SNAPSHOT_URL="http://db.test.azero.dev.s3-website.eu-central-1.amazonaws.com/latest-parity.html"
DB_PATH="chains/testnet/"
DB_ARG="--database paritydb"
TOP_BLOCK_SCRIPT="./.github/scripts/get_top_block.py"

while [[ $# -gt 0 ]]; do
    case "$1" in
        -p | --pruned)
            echo "Using pruned DB."
            DB_SNAPSHOT_URL="http://db.test.azero.dev.s3-website.eu-central-1.amazonaws.com/latest-parity-pruned.html"
            DB_ARG="--enable-pruning"
            shift;;
        *)
            echo "Unrecognized argument: $1"
            exit 1;;
    esac
done

initialize() {
    pip install substrate-interface
    mkdir -p "${BASE_PATH}"
}

get_snapshot () {
    echo "Downloading the snapshot...  "
    DB_SNAPSHOT_PATH=${BASE_PATH}/${DB_PATH}
    mkdir -p "${DB_SNAPSHOT_PATH}"
    pushd "${DB_SNAPSHOT_PATH}" > /dev/null

    set +e
    wget -q -O - ${DB_SNAPSHOT_URL} | tar xzf -
    if [[ 0 -ne $? ]]
    then
        error "Failed to download and unpack the snapshot."
    fi
    set -e
    popd > /dev/null
}

copy_chainspec () {
    echo "Copying the chainspec...   "
    cp ./bin/node/src/resources/testnet_chainspec.json "${CHAINSPEC}"
}

get_target_block() {
    echo "Determining target block...   "
    TARGET_BLOCK=`${TOP_BLOCK_SCRIPT} "wss://ws.test.azero.dev"`
}

get_current_block() {
    echo "Determining current block...   "
    CURRENT_BLOCK=`${TOP_BLOCK_SCRIPT} "ws://localhost:9944"`
}

initialize
copy_chainspec
get_snapshot

get_target_block

chmod +x aleph-node
./aleph-node \
    --chain "${CHAINSPEC}" \
    --base-path "${BASE_PATH}" \
    --rpc-port 9944 \
    --name sync-from-snapshot-tester \
    --bootnodes "${BOOT_NODES}" \
    --node-key-file "${BASE_PATH}/p2p_secret" \
    ${DB_ARG} \
    --no-mdns 1>/dev/null 2>/dev/null &

echo "Waiting a moment for the node to start up..."
sleep 5m

get_current_block
echo "Syncing to ${TARGET_BLOCK} starting at ${CURRENT_BLOCK}."

while [ $CURRENT_BLOCK -le $TARGET_BLOCK ]; do
    sleep 1m
    get_current_block
    echo "Sync status: ${CURRENT_BLOCK}/${TARGET_BLOCK}".
done
