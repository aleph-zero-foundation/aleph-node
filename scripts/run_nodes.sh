#!/usr/bin/env bash

# This script runs locally aleph-node consensus, ie set of aleph-node processes that are either
# validators (create blocks) or RPC nodes (read only nodes). By default, such consensus consist of
# one RPC node that is a bootnode, and 6 validator nodes.
#
# Before run, a chainspec is generated that is an initial testing AlephZero chain configuration,
# that is a starting point for all aleph-nodes. Together with the chainspec, we generate a keystore
# that consist of two types of keys:
#   * two session keys for each validator (one for authoring blocks called AURA key and one for
#     participating in AlephBFT consensus called ALEPH key)
#   * one key for participating in P2P network called p2p key
# The keystore is stored in a filesystem altogether with database, and this is called base path.
# Optionally, script can build testing aleph-node binary for you (short session meaning 30 blocks
# instead of normal 900 blocks session).
#
# Each validator has also an associated account on chain called Stash Account which is used for
# staking. For simplicity, such accounts seeds are generated from cardinal numbers //0, //1, //2, ...
# We assume RPC nodes have ids //0, //1, //2, ... //N-1, where N is number of RPC nodes, and
# validators have ids //N, //N+1, ...
# Obviously, this is just for testing reasons and such seeds must be never used in production.
# Also, each AlephNode chain has sudo account. Here we assume it is //Alice.
#
# Make sure a machine on which you're running your script has enough RAM memory. For testing nodes
# with empty db, a 1.5 GB per node would be enough.
#
# They are 3 set of ports you need to have opened and free on your machine in order to run consensus:
#  * RPC port - range [9944; 9954) - used for JSON RPC protocol
#  * P2p port - range [30333; 30343) - used for P2P peer network
#  * Validator port - range [30343; 30353) - used for consensus mechanism (AlephBFT)
#
# You need to have installed following prerequisites in order to use that script:
#   * jq
#
# This script also accepts env variables instead of arguments, see --help for details. All arguments
# are optional.

set -euo pipefail

# ------------------------ constants --------------------------------------

export ALEPH_NODE="target/release/aleph-node"
NODE_P2P_PORT_RANGE_START=30333
NODE_VALIDATOR_PORT_RANGE_START=30343
NODE_RPC_PORT_RANGE_START=9944

# ------------------------ argument parsing and usage -----------------------

script_path="${BASH_SOURCE[0]}"
script_dir=$(dirname "${script_path}")
aleph_node_root_dir=$(realpath "${script_dir}/..")
pushd "${aleph_node_root_dir}" > /dev/null
source ./scripts/common.sh

function usage(){
  cat << EOF
Usage:
   $0
    [-v|--validators VALIDATORS]
      number of validators to bootstrap and start
    [-n|--rpc-nodes RPC_NODES]
      number of RPC nodes to bootstrap and start
    [-p|--base-path BASE_PATH]
        if specified, use given base path (keystore, db, AlephBFT backups)
        if not specified, base path is ./run-nodes-local
    [--dont-bootstrap]
      set if you don't want to bootstrap chain, ie generate keystore and chainspec
    [--dont-build]
       set if you do not want to build testing aleph-node binary
    [--dont-delete-db]
      set to not delete database
    [--dont-remove-abtf-backups]
      set to not delete AlephBFT backups; by default they are removed since
      this script is intended to bootstrap chain by default, in which case you do not want to have
      them in 99% of scenarios
EOF
  exit 0
}

VALIDATORS=${VALIDATORS:-6}
RPC_NODES=${RPC_NODES:-1}
BASE_PATH=${BASE_PATH:-"./run-nodes-local"}
DONT_BOOTSTRAP=${DONT_BOOTSTRAP:-""}
DONT_BUILD_ALEPH_NODE=${DONT_BUILD_ALEPH_NODE:-""}
DONT_DELETE_DB=${DONT_DELETE_DB:-""}
DONT_REMOVE_ABFT_BACKUPS=${DONT_REMOVE_ABFT_BACKUPS:-""}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -v|--validators)
      VALIDATORS="$2"
      shift;shift
      ;;
    -n|--rpc-nodes)
      RPC_NODES="$2"
      shift;shift
      ;;
    -p|--base-path)
      BASE_PATH="$2"
      shift;shift
      ;;
    --dont-bootstrap)
      DONT_BOOTSTRAP="true"
      shift
      ;;
    --dont-build)
      DONT_BUILD_ALEPH_NODE="true"
      shift
      ;;
    --dont-delete-db)
      DONT_DELETE_DB="true"
      shift
      ;;
    --help)
      usage
      shift
      ;;
    --dont-remove-abft-backups)
      DONT_REMOVE_ABFT_BACKUPS="true"
      shift
      ;;
    *)
      error "Unrecognized argument $1!"
      ;;
  esac
done

# -----------------------  functions --------------------------------------
function get_backup_folders() {
  base_path="${1}"
  shift
  accounts_ids=("$@")

  declare -a backup_folders
  for account_id in ${accounts_ids[@]}; do
    maybe_backup_folder="${base_path}/${account_id}/backup-stash"
    if [[ -d "${maybe_backup_folder}" ]]; then
      backup_folders+=("${maybe_backup_folder}")
    fi
  done

  echo ${backup_folders[@]}
}

function get_ss58_address_from_seed() {
  local seed="$1"
  local aleph_node_path="$2"

  echo $(${aleph_node_path} key inspect --output-type json ${seed} | jq -r '.ss58Address')
}

function run_node() {
  local index="$1"
  local account_id="$2"
  local bootnode="$3"

  local node_name="node-${index}"
  local validator_port=$((NODE_VALIDATOR_PORT_RANGE_START + index))

  local node_args=(
    --validator
    --public-validator-addresses "127.0.0.1:${validator_port}"
    --chain "${BASE_PATH}/chainspec.json"
    --bootnodes "${bootnode}"
    --base-path "${BASE_PATH}/${account_id}"
    --name "${node_name}"
    --rpc-port $((NODE_RPC_PORT_RANGE_START + index))
    --port $((NODE_P2P_PORT_RANGE_START + index))
    --validator-port "${validator_port}"
    --node-key-file "${BASE_PATH}/${account_id}/p2p_secret"
    --backup-path "${BASE_PATH}/${account_id}/backup-stash"
    --rpc-cors=all
    --no-mdns
    --pool-limit 1024
    --db-cache 1024
    --runtime-cache-size 2
    --max-runtime-instances 8
    --enable-log-reloading
    --detailed-log-output
    -laleph-party=debug
    -laleph-network=debug
    -lnetwork-clique=debug
    -laleph-finality=debug
    -laleph-justification=debug
    -laleph-data-store=debug
    -laleph-updater=debug
    -laleph-metrics=debug
  )

  info "Running node ${index}..."
  ./target/release/aleph-node "${node_args[@]}"  2> "${BASE_PATH}/${node_name}.log" > /dev/null &
}

# ------------------------- input checks ----------------------------------

if [[ "${VALIDATORS}" -lt 4 ]]; then
  error "Number of validators should be at least 4!"
fi
if [[ "${RPC_NODES}" -lt 1 ]]; then
  error "Number of RPC nodes should be at least 1!"
fi
if [[ $(( VALIDATORS + RPC_NODES )) -gt 10 ]]; then
  info "Current number of validators is ${VALIDATORS} and RPC nodes is ${RPC_NODES}"
  error "Total number of validators and rpc nodes should not be greater than 10!"
fi
if [[ -z "${DONT_BOOTSTRAP}" && "${DONT_DELETE_DB}" == "true" ]]; then
  error "--dont-delete-db is set and --dont-bootstrap is not set
        When bootstraping chain, db must be deleted!
        Or if you want just to remove database, pass --dont-bootstrap to this script."
fi
if [[ "${DONT_BOOTSTRAP}" == "true" && ! -f "${BASE_PATH}/chainspec.json" ]]; then
  error "Flag --dont-bootstrap is set and there is no ${BASE_PATH}/chainspec.json file, maybe you
        forget to bootstrap chain?"
fi
if ! command -v jq &> /dev/null; then
    error "jq could not be found on PATH!"
fi

# ------------------- main script starts here ------------------------------

info "Starting $0"
info "Creating base path ${BASE_PATH} if it does not exist"
mkdir -p "${BASE_PATH}"
info "Stopping all current node processes"
if ! killall -9 aleph-node 2> /dev/null; then
  info "No aleph-node processes found."
fi

if [[ -z "${DONT_BUILD_ALEPH_NODE}" ]]; then
  info "Building testing aleph-node binary (short session)."
  cargo build --release -p aleph-node --features "short_session enable_treasury_proposals"
elif [[ ! -x "${ALEPH_NODE}" ]]; then
  error "${ALEPH_NODE} does not exist or it's not an executable file!"
fi

NUMBER_OF_NODES_TO_BOOTSTRAP=$(( VALIDATORS + RPC_NODES ))
info "Generating ${NUMBER_OF_NODES_TO_BOOTSTRAP} stash accounts identities."
declare -a rpc_node_account_ids
for i in $(seq 0 "$(( RPC_NODES - 1 ))"); do
  rpc_node_account_ids+=($(get_ss58_address_from_seed "//${i}" "${ALEPH_NODE}"))
done
declare -a validator_account_ids
for i in $(seq "${RPC_NODES}" "$(( NUMBER_OF_NODES_TO_BOOTSTRAP - 1 ))"); do
  validator_account_ids+=($(get_ss58_address_from_seed "//${i}" "${ALEPH_NODE}"))
done

info "Following identities were generated:"
info "RPC nodes: ${rpc_node_account_ids[@]}"
info "Validator nodes: ${validator_account_ids[@]}"

if [[ -z "${DONT_BOOTSTRAP}" ]]; then
  info "Bootstrapping chain for ${NUMBER_OF_NODES_TO_BOOTSTRAP} nodes."

  # space separated ids
  validator_ids_string="${validator_account_ids[*]}"
  # comma separated ids
  validator_ids_string="${validator_ids_string//${IFS:0:1}/,}"

  info "Creating chainspec and generating keystore for validators accounts."
  "${ALEPH_NODE}" bootstrap-chain --raw --base-path "${BASE_PATH}" --account-ids "${validator_ids_string}" --chain-type local > "${BASE_PATH}/chainspec.json"

  info "Generating keystores for ${RPC_NODES} RPC nodes"
  for i in $(seq 0 "$(( RPC_NODES - 1 ))"); do
    rpc_node_account_id="${rpc_node_account_ids[$i]}"
    "${ALEPH_NODE}" bootstrap-node --base-path "${BASE_PATH}/${rpc_node_account_id}" --account-id "${rpc_node_account_id}" --chain-type local > /dev/null
  done

  if [[ "${DONT_REMOVE_ABFT_BACKUPS}" == "true" ]]; then
    all_account_ids=(${validator_account_ids[@]} ${rpc_node_account_ids[@]})
    non_empty_backups=$(get_backup_folders "${BASE_PATH}" ${all_account_ids[@]})
    if [[ -n "${non_empty_backups}" ]]; then
      warning "Found following non-empty ABFT backups in base path:"
      warning "${non_empty_backups}"
      warning "In 99% you want them to be removed when bootstraping chain"
      warning "Re-run this script without flag --dont-remove-abtf-backups if you want them to be removed."
    fi
  fi
fi

info "Creating bootnode p2p multiaddress."
p2p_key_public=$("${ALEPH_NODE}" key inspect-node-key --file "${BASE_PATH}/${rpc_node_account_ids[0]}/p2p_secret")
bootnode_multiaddress="/dns4/localhost/tcp/$((NODE_P2P_PORT_RANGE_START))/p2p/${p2p_key_public}"
info "Bootnode p2p multiaddress is ${bootnode_multiaddress}"

if [[ -z "${DONT_DELETE_DB}" ]] ; then
  info "Removing database for all nodes (aka purging chain)."
  for i in $(seq 0 "$(( RPC_NODES - 1 ))"); do
    rpc_node_account_id=${rpc_node_account_ids[$i]}
    "${ALEPH_NODE}" purge-chain --base-path "${BASE_PATH}/${rpc_node_account_id}" --chain "${BASE_PATH}/chainspec.json" -y > /dev/null 2>&1
  done
  for i in $(seq 0 "$(( VALIDATORS - 1 ))"); do
    validator_account_id="${validator_account_ids[$i]}"
    "${ALEPH_NODE}" purge-chain --base-path "${BASE_PATH}/${validator_account_id}" --chain "${BASE_PATH}/chainspec.json" -y > /dev/null 2>&1
  done
fi

if [[ -z "${DONT_REMOVE_ABFT_BACKUPS}" ]]; then
  all_account_ids=(${validator_account_ids[@]} ${rpc_node_account_ids[@]})
  backups=$(get_backup_folders "${BASE_PATH}" ${all_account_ids[@]})
  if [[ "${backups[@]}" ]]; then
    info "Removing AlephBFT backups."
    echo "${backups[@]}" | xargs rm -rf
  fi
fi

for i in $(seq 0 "$(( RPC_NODES - 1 ))"); do
  rpc_node_account_id=${rpc_node_account_ids[$i]}
  run_node "$i" "${rpc_node_account_id}" "${bootnode_multiaddress}"
done

for i in $(seq 0 "$(( VALIDATORS - 1 ))"); do
  validator_account_id=${validator_account_ids[$i]}
  run_node $(( i + RPC_NODES )) "${validator_account_id}" "${bootnode_multiaddress}"
done

popd > /dev/null
exit 0
