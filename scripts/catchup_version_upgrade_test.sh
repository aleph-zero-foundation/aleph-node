#!/bin/bash

set -euo pipefail

INIT_BLOCK=${INIT_BLOCK:-3}
UPGRADE_BLOCK=${UPGRADE_BLOCK:-31}
UPGRADE_VERSION=${UPGRADE_VERSION:-1}
NODES=${NODES:-"Node1:Node2"}
PORTS=${PORTS:-9934:9935}
UPGRADE_BEFORE_DISABLE=${UPGRADE_BEFORE_DISABLE:-false}
SEED=${SEED:-"//Alice"}
ALL_NODES=${ALL_NODES:-"Node0:Node1:Node2:Node3:Node4"}
ALL_NODES_PORTS=${ALL_NODES_PORTS:-"9933:9934:9935:9936:9937"}
WAIT_BLOCKS=${WAIT_BLOCKS:-30}
EXT_STATUS=${EXT_STATUS:-"in-block"}

source ./scripts/common.sh

function initialize {
    wait_for_finalized_block $1 $2 $3
}

function set_upgrade_session {
    local session=$1
    local version=$2
    local validator=$3
    local port=$4
    local seed=$5
    local status=$6

    docker run --rm --network container:$validator cliain:latest --node ws://127.0.0.1:$port --seed $seed version-upgrade-schedule --version $version --session $session --expected-state $status
}

function check_if_disconnected() {
    local -n nodes=$1
    local -n ports=$2

    log "checking if nodes are disconnected"

    for i in "${!nodes[@]}"; do
        local node=${nodes[$i]}
        local port=${ports[$i]}

        log "checking if node $node is disconnected"

        last_finalized=$(get_best_finalized $node $port)
        log "last finalized block at node $node is $last_finalized"

        last_block=$(get_last_block $node $port)
        log "last block at node $node is $last_block"

        # what else we can do?
        log "sleeping for 20 seconds"
        sleep 20

        new_finalized=$(get_best_finalized $node $port)
        log "newest finalized block at node $node after waiting is $new_finalized"

        if [[ $(($new_finalized - $last_finalized)) -ge 1 ]]; then
            log "somehow a disconnected node $node was able to finalize new blocks"
            exit -1
        fi
    done
}

function connect_nodes {
    local -n nodes=$1
    for node in ${nodes[@]}; do
        docker network connect main-network $node
    done
}

function disconnect_nodes {
    local -n nodes=$1

    for node in ${nodes[@]}; do
        log "disconnecting node $node..."
        docker network disconnect main-network $node
        log "node $node disconnected"
    done
}

into_array $NODES
NODES=(${result[@]})

into_array $PORTS
PORTS=(${result[@]})

into_array $ALL_NODES
ALL_NODES=(${result[@]})

into_array "$ALL_NODES_PORTS"
ALL_NODES_PORTS=(${result[@]})

log "initializing nodes..."
DOCKER_COMPOSE=./docker/docker-compose.bridged.yml ./.github/scripts/run_consensus.sh 1>&2
sleep 10
log "awaiting finalization of $INIT_BLOCK blocks..."
initialize $INIT_BLOCK "Node0" 9933
log "nodes initialized"

last_block=$(get_last_block "Node0" 9933)
block_for_upgrade=$(($UPGRADE_BLOCK + $last_block))
if [[ $UPGRADE_BEFORE_DISABLE = true ]]; then
    log "setting upgrade at $block_for_upgrade block for version $UPGRADE_VERSION before disconnecting"
    set_upgrade_session $block_for_upgrade $UPGRADE_VERSION "Node0" 9943 $SEED $EXT_STATUS
fi

log "disconnecting nodes..."
disconnect_nodes NODES
log "verifying if nodes are properly disconnected..."
check_if_disconnected NODES PORTS
log "nodes disconnected"

last_block=$(get_last_block "Node0" 9933)
block_for_upgrade=$(($UPGRADE_BLOCK + $last_block))
if [[ $UPGRADE_BEFORE_DISABLE = false ]]; then
    log "setting upgrade at $block_for_upgrade block for version $UPGRADE_VERSION"
    set_upgrade_session $block_for_upgrade $UPGRADE_VERSION "Node0" 9943 $SEED $EXT_STATUS
fi

last_block=$(get_last_block "Node0" 9933)
awaited_block=$(($WAIT_BLOCKS+$block_for_upgrade))
log "awaiting block $awaited_block"
wait_for_block $awaited_block "Node0" 9933
log "awaiting finished"

log "connecting nodes..."
connect_nodes NODES
log "nodes connected"

last_block=$(get_last_block "Node0" 9933)
log "checking finalization..."
check_finalization $(($awaited_block+1)) ALL_NODES ALL_NODES_PORTS
log "finalization checked"

exit $?
