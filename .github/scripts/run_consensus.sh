#!/usr/bin/env bash

# This script bootstraps and runs aleph-node consensus for e2e tests.
# It is not intended to by used by end user, use [run_nodes.sh](../../scripts/run_nodes.sh) instead.
# This script has one output, which is BOOTNODE_PEER_ID environment variable - a p2p mulitaddress
# of the bootnode
# Known issues:
#   AlephBFT backups are not removed by this script, as docker operates as root, and this
#   script does not. As a consequence, you might face a silent finalization failures when
#   running this script on the same machine more than once, e.g. when running different
#   consensus size.


set -euo pipefail

# ------------------------ constants --------------------------------------

NODE_COUNT=${NODE_COUNT:-6}
MIN_VALIDATOR_COUNT=${MIN_VALIDATOR_COUNT:-4}
DOCKER_COMPOSE=${DOCKER_COMPOSE:-docker/docker-compose.yml}
OVERRIDE_DOCKER_COMPOSE=${OVERRIDE_DOCKER_COMPOSE:-""}
NODE_IMAGE=${NODE_IMAGE:-"aleph-node:latest"}

# ------------------------ argument parsing and usage -----------------------

function usage {
   cat << EOF
Usage
  $0
    [-n|--node-count NODE_COUNT]
      number of nodes to run
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case $1 in
  -h|--help)
    usage
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

# ---------------------------- functions --------------------------------------

function generate_account_ids() {
  local validators_count="$1"

  echo "Generating one RPC node and ${validators_count} validators accounts ids..." >&2
  local account_ids=()
  for node_index in $(seq 0 "${validators_count}"); do
    echo "Generating account ${node_index} from key //${node_index}" >&2
    account_ids+=($(docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" "${NODE_IMAGE}" \
       -c "aleph-node key inspect //$node_index | grep \"SS58 Address:\" | awk \"{print \\\$3;}\""))
  done
  echo ${account_ids[*]}
}

function generate_chainspec() {
  local account_ids=("$@")

  # First array element is RPC node, so not a validator
  local validators=${account_ids[@]:1}
  # comma separated ids
  validator_ids="${validators//${IFS:0:1}/,}"

  echo "Generate chainspec and keystores with sudo account //Alice for below validators..."
  echo "${validator_ids}"
  docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=debug "${NODE_IMAGE}" \
  -c "aleph-node bootstrap-chain --base-path /data --account-ids "${validator_ids}" > /data/chainspec.json"

  echo "Generating keystore for RPC node ${account_ids[0]}..."
  docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" -e RUST_LOG=debug "${NODE_IMAGE}" \
  -c "aleph-node bootstrap-node --base-path /data/${account_ids[0]} --account-id ${account_ids[0]}" > /dev/null
}

function generate_bootnode_peer_id() {
  local bootnode_account="$1"

  echo "Generate bootnode peer id..."
  export BOOTNODE_PEER_ID=$(docker run --rm -v $(pwd)/docker/data:/data --entrypoint "/bin/sh" \
     -e RUST_LOG=info "${NODE_IMAGE}" \
     -c "aleph-node key inspect-node-key --file /data/${bootnode_account}/p2p_secret")
}

function run_containers() {
  local authorities_count="$1"
  local docker_compose_file="$2"
  local override_file="$3"

  echo "Running $(( authorities_count + 1)) containers..."
  local containers=()

  for index in $(seq 0 "${authorities_count}"); do
    containers+=("Node${index}")
  done
  if [[ -z ${override_file} ]]; then
    docker-compose -f "${docker_compose_file}" up -d "${containers[@]}"
  else
    docker-compose -f "${docker_compose_file}" -f "${override_file}" up -d "${containers[@]}"
  fi
}

# --------------------------------- main script --------------------------------------------

echo "Starting $0"

script_path="${BASH_SOURCE[0]}"
script_dir=$(dirname "${script_path}")
aleph_node_root_dir=$(realpath "${script_dir}/../..")
pushd "${aleph_node_root_dir}" > /dev/null

if docker inspect ${NODE_IMAGE} > /dev/null; then
  echo "aleph-node image tag ${NODE_IMAGE} found locally"
else
  echo "${NODE_IMAGE} not found locally."
  echo "Build image first with:"
  echo "docker build -t ${NODE_IMAGE} -f docker/Dockerfile ."
  exit 1
fi

mkdir -p docker/data/
echo "Warning: if you run this script on the same machine more then once, and finalization does not work, remove docker/data."
global_account_ids=$(generate_account_ids ${NODE_COUNT})
generate_chainspec ${global_account_ids[@]}
generate_bootnode_peer_id ${global_account_ids[0]}
run_containers "${NODE_COUNT}" "${DOCKER_COMPOSE}" "${OVERRIDE_DOCKER_COMPOSE}"
echo "Finished $0"
popd > /dev/null

exit 0
