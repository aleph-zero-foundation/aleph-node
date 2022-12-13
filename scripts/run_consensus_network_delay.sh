#!/bin/env bash

set -euo pipefail

source ./scripts/common.sh

function usage(){
    cat << EOF
Usage:
  $0
     --network-delays "500:300"
        list of delays for each node in ms; default="500:500:500:500:500"
    --no-build-image
        skip docker image build
    --nodes "Node0:9933:Node1:9934"
        list of pairs node:rpc_port; default="Node0:9933:Node1:9934:Node2:9935:Node3:9936:Node4:9937"
    --check-block number
        check finalization for a given block number, 0 means no-check; default=42
EOF
    exit 0
}

function build_test_image() {
    docker build -t aleph-node:network_tests -f docker/Dockerfile.network_tests .
}

function set_network_delay() {
    local node=$1
    local delay=$2

    log "setting network delay for node $node"
    docker exec $node tc qdisc add dev eth1 root netem delay ${delay}ms
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --network-delays)
            NETWORK_DELAYS="$2"
            shift;shift
            ;;
        --no-build-image)
            BUILD_IMAGE=false
            shift
            ;;
        --nodes)
            NODES="$2"
            shift;shift
            ;;
        --check-block)
            CHECK_BLOCK_FINALIZATION="$2"
            shift;shift
            ;;
        --help)
            usage
            shift
            ;;
        *)
            error "Unrecognized argument $1!"
            ;;
    esac
done

NETWORK_DELAYS=${NETWORK_DELAYS:-"500:500:500:500:500"}
BUILD_IMAGE=${BUILD_IMAGE:-true}
NODE_PAIRS=${NODES:-"Node0:9933:Node1:9934:Node2:9935:Node3:9936:Node4:9937"}
NODES_PORTS=${NODES_PORTS:-"9933:9934:9935:9936:9937"}
CHECK_BLOCK_FINALIZATION=${CHECK_BLOCK_FINALIZATION:-44}

into_array $NETWORK_DELAYS
NETWORK_DELAYS=(${result[@]})

into_array $NODE_PAIRS
NODE_PAIRS=(${result[@]})
NODES=()
NODES_PORTS=()
for ((i=0; i<${#NODE_PAIRS[@]}; i+=2)); do
    node=${NODE_PAIRS[$i]}
    port=${NODE_PAIRS[(($i + 1))]}

    NODES+=($node)
    NODES_PORTS+=($port)
done


if [[ "$BUILD_IMAGE" = true ]]; then
    log "building custom docker image for network tests"
    build_test_image
fi

log "starting network"
OVERRIDE_DOCKER_COMPOSE=./docker/docker-compose.network_tests.yml DOCKER_COMPOSE=./docker/docker-compose.bridged.yml ./.github/scripts/run_consensus.sh 1>&2
log "network started"

for i in "${!NODES[@]}"; do
    node=${NODES[$i]}
    delay=${NETWORK_DELAYS[$i]}
    log "setting network delay for node $node to ${delay}ms"

    set_network_delay $node $delay
done

if [[ $CHECK_BLOCK_FINALIZATION -gt 0 ]]; then
    log "checking finalization"
    check_relative_finalization $CHECK_BLOCK_FINALIZATION NODES NODES_PORTS
    log "finalization checked"
fi

log "done"

exit 0
