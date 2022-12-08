#!/bin/env bash

set -euo pipefail

source ./scripts/common.sh

function usage(){
    cat << EOF
Usage:
  $0
  This script allows you to run a custom .js script using the synthetic-network network simulation tool.
  IMPORTANT: first you need to call 'scripts/run_consensus_synthetic-network.sh' and let it run in background.
             It spawns docker-compose configured with synthetic-network.
             It requires node.js to run.
    --commit 72bbb4fde915e4132c19cd7ce3605364abac58a5
        commit hash used to build synthetic-network, default is 72bbb4fde915e4132c19cd7ce3605364abac58a5
    --script-path scripts/vendor/synthetic-network/frontend/udp_rate_sine_demo.js
        path to a synthetic-network scrypt. Default is a demo scripts/vendor/synthetic-network/frontend/udp_rate_sine_demo.js
        from the synthetic-network repo. Please consult synthetic-network repo for details: https://github.com/daily-co/synthetic-network
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --commit)
            GIT_COMMIT="$2"
            shift;shift
            ;;
        --script-path)
            SCRIPT_PATH="$2"
            shift;shift
            ;;
        --help)
            usage
            shift
            ;;
        *)
            break
            ;;
    esac
done

GIT_COMMIT=${GIT_COMMIT:-72bbb4fde915e4132c19cd7ce3605364abac58a5}
SCRIPT_PATH=${SCRIPT_PATH:-scripts/vendor/synthetic-network/frontend/udp_rate_sine_demo.js}
SCRIPT_PATH=$(realpath $SCRIPT_PATH)

TMPDIR="$(dirname $0)/vendor"
mkdir -p $TMPDIR
log "created a temporary folder at $TMPDIR"

log "cloning synthetic-network's git repo"
cd $TMPDIR
if [[ ! -d ./synthetic-network ]]; then
    git clone https://github.com/daily-co/synthetic-network.git
fi
cd synthetic-network
git fetch origin
git checkout $GIT_COMMIT
cd frontend

log "running .js script"
node $SCRIPT_PATH ${@:1}

exit 0
