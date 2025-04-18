#!/bin/bash
set -euox pipefail
echo "Starting db sync test."

PARITY_DB="false"
PRUNING="false"
ENV="mainnet"
SNAPSHOT_DAY=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --parity-db)
            echo "Using ParityDB."
            PARITY_DB="true"
            shift;;
        --pruned)
            echo "Using pruned DB."
            PRUNING="true"
            shift;;
        --mainnet)
            ENV="mainnet"
            shift;;
        --testnet)
            ENV="testnet"
            shift;;
        --snapshot-day)
            SNAPSHOT_DAY="$2"
            shift; shift
            ;;
        *)
            echo "Unrecognized argument: $1"
            exit 1;;
    esac
done

if [[ "${PRUNING}" == "false" && "${PARITY_DB}" == "true" ]]; then
    echo "Error! Passed '--parity-db' without '--pruned'."
    echo "That is an unsupported argument combination."
    exit 1
fi

BASE_PATH="running/"
CHAINSPEC="${BASE_PATH}/chainspec.json"
TOP_BLOCK_SCRIPT="./.github/scripts/get_top_block.py"

if [[ "${ENV}" == "mainnet" ]]; then
    SOURCE_CHAINSPEC="./bin/node/src/resources/mainnet_chainspec.json"
    BOOT_NODES=/dns4/bootnode-eu-central-1-0.azero.dev/tcp/30333/p2p/12D3KooWEF1Eo7uFZWdqFsTPP7CehpRt5NeXFwCe3157qpoU5aqd/dns4/bootnode-eu-central-1-1.azero.dev/tcp/30333/p2p/12D3KooWSeKnKHwumcVuWz2g5wn5xyWZpZJzuZXHJrEdpi8bj4HR/dns4/bootnode-us-east-1-0.azero.dev/tcp/30333/p2p/12D3KooWFQSGvQii2gRGB5T4M6TXhM83JV4bTEhubCBpdoR6Rkwk/dns4/bootnode-us-east-1-1.azero.dev/tcp/30333/p2p/12D3KooWSX2TbzpengsKsXdNPs6g2aQpp91qduL5FPax2SqgCaxa
    DB_PATH="chains/mainnet/"
    TARGET_CHAIN="wss://ws.azero.dev"
    BASE_SNAPSHOT_URL="https://azero-snapshots.dev/mainnet"
else
    SOURCE_CHAINSPEC="./bin/node/src/resources/testnet_chainspec.json"
    BOOT_NODES=/dns4/bootnode-eu-central-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L/dns4/bootnode-us-east-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWSv1nApKkcnq8ZVHJQLK5GJ4NKS9ebag9QrRTzksLTGUy
    DB_PATH="chains/testnet/"
    TARGET_CHAIN="wss://ws.test.azero.dev"
    BASE_SNAPSHOT_URL="https://azero-snapshots.dev/testnet"
fi

declare -a DB_ARG
S3_SNAPSHOT_PREFIX=""
if [[ "${PARITY_DB}" == "true" && "${PRUNING}" == "true" ]]; then
    DB_ARG+=("--database paritydb")
    DB_ARG+=("--enable-pruning")
    S3_SNAPSHOT_PREFIX="paritydb-pruned"
fi
if [[ "${PARITY_DB}" == "false" && "${PRUNING}" == "true"  ]]; then
    DB_ARG+=("--enable-pruning")
    S3_SNAPSHOT_PREFIX="rocksdb-pruned"
fi
if [[ "${PARITY_DB}" == "false" && "${PRUNING}" == "false" ]]; then
    S3_SNAPSHOT_PREFIX="rocksdb"
fi

if [[ -z "${SNAPSHOT_DAY}" ]]; then
    SNAPSHOT_DAY=$(date "+%Y-%m-%d")
fi

DB_SNAPSHOT_URL="${BASE_SNAPSHOT_URL}/${S3_SNAPSHOT_PREFIX}/db_${SNAPSHOT_DAY}.tar.gz"

initialize() {
    pip install substrate-interface
    mkdir -p "${BASE_PATH}"
}

get_snapshot () {
    echo "Downloading the snapshot...  "
    DB_SNAPSHOT_PATH=${BASE_PATH}/${DB_PATH}
    mkdir -p "${DB_SNAPSHOT_PATH}"
    pushd "${DB_SNAPSHOT_PATH}" > /dev/null

    wget -q -O - "${DB_SNAPSHOT_URL}" | tar xzf -
    if [[ 0 -ne $? ]]
    then
        error "Failed to download and unpack the snapshot."
    fi
    popd > /dev/null
}

copy_chainspec () {
    echo "Copying the chainspec...   "
    cp "${SOURCE_CHAINSPEC}" "${CHAINSPEC}"
}

get_target_block() {
    echo "Determining target block...   "
    TARGET_BLOCK=`${TOP_BLOCK_SCRIPT} "${TARGET_CHAIN}"`
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
    ${DB_ARG[*]} \
    --no-mdns 1>/dev/null 2> "${BASE_PATH}/aleph-node.log" &
ALEPH_NODE_PID=$!

get_current_block
echo "Syncing to ${TARGET_BLOCK} starting at ${CURRENT_BLOCK}."

while [ $CURRENT_BLOCK -le $TARGET_BLOCK ]; do
    sleep 1m
    get_current_block
    echo "Sync status: ${CURRENT_BLOCK}/${TARGET_BLOCK}".
done

kill -9 $ALEPH_NODE_PID

