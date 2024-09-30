#!/bin/bash
set -euox pipefail
echo "Starting db sync test."

PARITY_DB="false"
PRUNING="false"
ENV="mainnet"
SNAPSHOT_DAY=""
MARK_SNAPSHOT_AS_LATEST=""
S3_BUCKET=""

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
        --mark-snapshot-as-latest)
            MARK_SNAPSHOT_AS_LATEST="true"
            shift;;
        *)
            echo "Unrecognized argument: $1"
            exit 1;;
    esac
done

if [[ "${PRUNING}" == "true" && "${PARITY_DB}" == "false" ]]; then
    echo "Error! Passed '--pruned' without '--parity-db'"
    echo "That is an unsupported argument combination."
    exit 1
fi

BASE_PATH="running/"
CHAINSPEC="${BASE_PATH}/chainspec.json"
TOP_BLOCK_SCRIPT="./.github/scripts/get_top_block.py"

if [[ "${ENV}" == "mainnet" ]]; then
    SOURCE_CHAINSPEC="./bin/node/src/resources/mainnet_chainspec.json"
    BOOT_NODES=/dns4/bootnode-eu-central-1-0.azero.dev/tcp/30333/p2p/12D3KooWEF1Eo7uFZWdqFsTPP7CehpRt5NeXFwCe3157qpoU5aqd/dns4/bootnode-eu-west-1-0.azero.dev/tcp/30333/p2p/12D3KooWPhi8Qvzvc8iJ4CeQj2vptjc5FTrodKPmra1NS1qfftjr/dns4/bootnode-eu-west-2-0.azero.dev/tcp/30333/p2p/12D3KooWDfUzU64WURE77tXYM9H94xQFAEL6ULQYhzegKsZXjEkC/dns4/bootnode-us-east-1-0.azero.dev/tcp/30333/p2p/12D3KooWFQSGvQii2gRGB5T4M6TXhM83JV4bTEhubCBpdoR6Rkwk/dns4/bootnode-us-east-2-0.azero.dev/tcp/30333/p2p/12D3KooWJnEGVVmnXhVNxV6KWe3EsDPNvPFNcYbQ6amFVGECVAGB
    DB_PATH="chains/mainnet/"
    TARGET_CHAIN="wss://ws.azero.dev"
    S3_URL="http://db.azero.dev.s3-website.eu-central-1.amazonaws.com"
    S3_BUCKET="db.azero.dev"
else
    SOURCE_CHAINSPEC="./bin/node/src/resources/testnet_chainspec.json"
    BOOT_NODES=/dns4/bootnode-eu-central-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L/dns4/bootnode-eu-west-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k/dns4/bootnode-eu-west-2-0.test.azero.dev/tcp/30333/p2p/12D3KooWAkqYFFKMEJn6fnPjYnbuBBsBZq6fRFJZYR6rxnuCZWCC/dns4/bootnode-us-east-1-0.test.azero.dev/tcp/30333/p2p/12D3KooWQFkkFr5aM5anGEiUCQiGUdRyWgrdpvSjBgWAUS9srLE4/dns4/bootnode-us-east-2-0.test.azero.dev/tcp/30333/p2p/12D3KooWD5s2dkifJua69RbLwEREDdJjsNHvavNRGxdCvzhoeaLc
    DB_PATH="chains/testnet/"
    TARGET_CHAIN="wss://ws.test.azero.dev"
    S3_URL="http://db.test.azero.dev.s3-website.eu-central-1.amazonaws.com"
    S3_BUCKET="db.test.azero.dev"
fi

declare -a DB_ARG
S3_SNAPSHOT_PREFIX=""
LATEST_SNAPSHOT_NAME=""
if [[ "${PARITY_DB}" == "true" ]]; then
    DB_ARG+=("--database paritydb")
    S3_SNAPSHOT_PREFIX="db_backup_parity"
    LATEST_SNAPSHOT_NAME="latest-parity.html"
fi
if [[ "${PRUNING}" == "true" ]]; then
    DB_ARG+=("--enable-pruning")
    S3_SNAPSHOT_PREFIX="db_backup_parity_pruned"
    LATEST_SNAPSHOT_NAME="latest-parity-pruned.html"
fi
if [[ "${PARITY_DB}" != "true" && "${PRUNING}" != "true" ]]; then
    S3_SNAPSHOT_PREFIX="db_backup"
    LATEST_SNAPSHOT_NAME="latest.html"
fi

if [[ -z "${SNAPSHOT_DAY}" ]]; then
    SNAPSHOT_DAY=$(date "+%Y-%m-%d")
fi

DB_SNAPSHOT_URL="${S3_URL}/${SNAPSHOT_DAY}/${S3_SNAPSHOT_PREFIX}_${SNAPSHOT_DAY}.tar.gz"

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

if [[ "${MARK_SNAPSHOT_AS_LATEST}" == "true" ]]; then
  echo "<meta http-equiv=\"refresh\" content=\"0;url=${S3_URL}/${SNAPSHOT_DAY}/${S3_SNAPSHOT_PREFIX}_${SNAPSHOT_DAY}.tar.gz\">" | \
    aws s3 cp - "s3://${S3_BUCKET}/${LATEST_SNAPSHOT_NAME}" \
      --website-redirect "${S3_URL}/${SNAPSHOT_DAY}/${S3_SNAPSHOT_PREFIX}_${SNAPSHOT_DAY}.tar.gz"
fi

kill -9 $ALEPH_NODE_PID

