#!/bin/bash

set -euo pipefail

# default node count
# change when increasing the number of node containers
NODE_COUNT=5

export NODE_IMAGE=aleph-node:latest

mkdir -p docker/data/

function usage {
   cat << EOF
Usage
  $0
    --node-count, -n
      number of nodes to run
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case $1 in
  -h|--help)
    usage
    exit 0
    ;;
  -m|--min-validator-count)
    MIN_VALIDATOR_COUNT="$2"
    shift 2
    ;;
  -n|--node-count)
    NODE_COUNT="$2"
    shift 2
    ;;
  *)
    echo "Unrecognized argument $1!"
    usage
    exit 1
    ;;
  esac
done

export NODE_COUNT

function generate_authorities {
  local authorities_count="$1"

  echo "Generating ${authorities_count} authorities accounts ids..." >&2
  declare -a account_ids
  for node_index in $(seq 0 $((authorities_count - 1))); do
    echo "Generating authority ${node_index} from key //${node_index}" >&2
    account_ids+=($(docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" "${NODE_IMAGE}" -c "aleph-node key inspect //$node_index | grep \"SS58 Address:\" | awk \"{print \\\$3;}\""))
  done
  echo "${account_ids[*]}"
}

function generate_chainspec {
  local authorities="$1"
  local min_validator_count="$2"

  # comma separated ids
  validator_ids="${authorities//${IFS:0:1}/,}"

  echo "Generate chainspec and keystores with sudo account //Alice ..."
  docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=debug "${NODE_IMAGE}" -c \
  "aleph-node bootstrap-chain --base-path /data --account-ids $validator_ids \
  --min-validator-count "${min_validator_count}" > /data/chainspec.json"
}

function generate_bootnode_peer_id {
  local bootnode_account="$1"

  echo "Generate bootnode peer id..."
  export BOOTNODE_PEER_ID=$(docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=info "${NODE_IMAGE}" -c "aleph-node key inspect-node-key --file /data/$bootnode_account/p2p_secret")
}

function run_containers {
  local authorities_count="$1"

  echo "Running ${authorities_count} containers..."
  docker-compose -f docker/docker-compose.yml up -d
}

authorities=$(generate_authorities ${NODE_COUNT})
generate_chainspec "${authorities[@]}" "${MIN_VALIDATOR_COUNT}"
generate_bootnode_peer_id ${authorities[0]}
run_containers ${NODE_COUNT}

exit $?
