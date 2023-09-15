#!/bin/bash

function usage(){
  echo "Usage:
      ./run_nodes.sh [-v N_VALIDATORS] [-n N_NON_VALIDATORS] [-b false] [-p BASE_PATH] [-l N_LISTENERES] [-r false] [ALEPH_NODE_ARG]...
  where 2 <= N_VALIDATORS <= N_VALIDATORS + N_NON_VALIDATORS + N_LISTENERES <= 10
  (by default, N_VALIDATORS=4, N_NON_VALIDATORS=0, N_LISTENERES=0 and BASE_PATH=/tmp)"
}

N_VALIDATORS=4
N_NON_VALIDATORS=0
N_LISTENERES=0
BUILD_ALEPH_NODE='true'
BASE_PATH='/tmp'
BOOTSTRAP='true'

while getopts "v:n:b:p:l:r:" flag
do
  case "${flag}" in
    v) N_VALIDATORS=${OPTARG};;
    n) N_NON_VALIDATORS=${OPTARG};;
    b) BUILD_ALEPH_NODE=${OPTARG};;
    p) BASE_PATH=${OPTARG};;
    l) N_LISTENERES=${OPTARG};;
    r) BOOTSTRAP=${OPTARG};;
    *)
      usage
      exit
      ;;
  esac
done

shift $((OPTIND-1))

killall -9 aleph-node

set -e

clear


if $BUILD_ALEPH_NODE ; then
  cargo build --release -p aleph-node --features "short_session enable_treasury_proposals"
fi

declare -a account_ids
for i in $(seq 0 "$(( N_VALIDATORS + N_NON_VALIDATORS - 1 ))"); do
  account_ids+=($(./target/release/aleph-node key inspect "//$i" | grep "SS58 Address:" | awk '{print $3;}'))
done
validator_ids=("${account_ids[@]::N_VALIDATORS}")
# space separated ids
validator_ids_string="${validator_ids[*]}"
# comma separated ids
validator_ids_string="${validator_ids_string//${IFS:0:1}/,}"

if $BOOTSTRAP ; then
  echo "Bootstrapping chain for nodes 0..$((N_VALIDATORS - 1))"
  ./target/release/aleph-node bootstrap-chain --raw --base-path "$BASE_PATH" --account-ids "$validator_ids_string" --chain-type local > "$BASE_PATH/chainspec.json"

  for i in $(seq "$N_VALIDATORS" "$(( N_VALIDATORS + N_NON_VALIDATORS - 1 ))"); do
    echo "Bootstrapping node $i"
    account_id=${account_ids[$i]}
    ./target/release/aleph-node bootstrap-node --base-path "$BASE_PATH/$account_id" --account-id "$account_id" --chain-type local
  done
fi

addresses=()
for i in $(seq 0 "$(( N_VALIDATORS + N_NON_VALIDATORS - 1 ))"); do
    pk=$(./target/release/aleph-node key inspect-node-key --file $BASE_PATH/${account_ids[$i]}/p2p_secret)
    addresses+=("/dns4/localhost/tcp/$((30334+i))/p2p/$pk")
done

bootnodes=""
for i in 0 1; do
    bootnodes+="${addresses[i]} "
done

run_node() {
  i=$1
  is_validator=$2
  auth=node-$i
  account_id=${account_ids[$i]}
  validator_port=$((30343 + i))

  [[ $is_validator = true ]] && validator=--validator || validator=""

  if $BOOTSTRAP ; then
    ./target/release/aleph-node purge-chain --base-path $BASE_PATH/$account_id --chain $BASE_PATH/chainspec.json -y
  fi
  ./target/release/aleph-node \
    $validator \
    --chain $BASE_PATH/chainspec.json \
    --base-path $BASE_PATH/$account_id \
    --name $auth \
    --rpc-port $((9943 + i)) \
    --port $((30334 + i)) \
    --bootnodes $bootnodes \
    --node-key-file $BASE_PATH/$account_id/p2p_secret \
    --backup-path $BASE_PATH/$account_id/backup-stash \
    --execution Native \
    --rpc-cors=all \
    --no-mdns \
    --public-validator-addresses 127.0.0.1:${validator_port} \
    --validator-port ${validator_port} \
    --detailed-log-output \
    -laleph-party=debug \
    -laleph-network=debug \
    -lnetwork-clique=debug \
    -laleph-finality=debug \
    -laleph-justification=debug \
    -laleph-data-store=debug \
    -laleph-updater=debug \
    -laleph-metrics=debug \
    2> $auth.log > /dev/null & \
}

for i in $(seq 0 "$(( N_VALIDATORS + N_NON_VALIDATORS - 1 ))"); do
  run_node "$i" true
done

for i in $(seq "$(( N_VALIDATORS + N_NON_VALIDATORS))" "$(( N_VALIDATORS + N_NON_VALIDATORS - 1 + N_LISTENERES))"); do
  run_node "$i" false
done
