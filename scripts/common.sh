#!/bin/env bash

function log() {
    echo $1 1>&2
}

function into_array() {
    result=()
    local tmp=$IFS
    IFS=:
    for e in $1; do
        result+=($e)
    done
    IFS=$tmp
}

function check_finalization() {
    local block_to_check=$1
    local -n nodes=$2
    local -n ports=$3

    log "checking finalization for block $block_to_check"

    for i in "${!nodes[@]}"; do
        local node=${nodes[$i]}
        local rpc_port=${ports[$i]}

        log "checking finalization at node $node"
        wait_for_finalized_block $block_to_check $node $rpc_port
    done
}

function check_relative_finalization_at_node() {
    local node=$1
    local rpc_port=$2
    local awaited_blocks=$3

    local last_block=$(get_last_block $node $rpc_port)
    local awaited_finalized=$(($last_block+$awaited_blocks))

    log "Last block seen at node $node was $last_block, awaiting block $awaited_finalized to be finalized"

    wait_for_finalized_block $awaited_finalized $node $rpc_port
}

function check_relative_finalization() {
    local awaited_blocks=$1
    local -n nodes=$2
    local -n ports=$3

    log "checking finalization for $awaited_blocks block(s) in the future"

    for i in "${!nodes[@]}"; do
        local node=${nodes[$i]}
        local rpc_port=${ports[$i]}

        log "checking finalization at node $node (${node}:$rpc_port)"
        check_relative_finalization_at_node $node $rpc_port $awaited_blocks
    done
}

function get_best_finalized() {
    local validator=$1
    local rpc_port=$2

    local best_finalized=$(VALIDATOR=$validator RPC_HOST="127.0.0.1" RPC_PORT=$rpc_port ./.github/scripts/check_finalization.sh | sed 's/Last finalized block number: "\(.*\)"/\1/')
    printf "%d" $best_finalized
}

function wait_for_finalized_block() {
    local block_to_be_finalized=$1
    local node=$2
    local port=$3

    while [[ $(get_best_finalized $node $port) -le $block_to_be_finalized ]]; do
        sleep 3
    done
}

function wait_for_block() {
    local block=$1
    local validator=$2
    local rpc_port=$3

    local last_block=""
    while [[ -z "$last_block" ]]; do
        last_block=$(docker run --rm --network container:$validator appropriate/curl:latest \
                            -H "Content-Type: application/json" \
                            -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlockHash", "params": '$block'}' http://127.0.0.1:$rpc_port | jq '.result')
    done
}

function retrieve_last_block() {
    local validator=$1
    local rpc_port=$2

    docker run --rm --network container:$validator appropriate/curl:latest \
           -H "Content-Type: application/json" \
           -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock"}' http://127.0.0.1:$rpc_port | jq '.result.block.header.number'
}

function get_last_block() {
    local validator=$1
    local rpc_port=$2

    local last_block=""
    while [[ -z "$last_block" ]]; do
        last_block=$(retrieve_last_block $validator $rpc_port)
        sleep 1
    done
    printf "%d" $last_block
}
