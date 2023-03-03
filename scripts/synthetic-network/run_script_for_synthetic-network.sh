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
    --script-path scripts/vendor/synthetic-network/frontend/udp_rate_sine_demo.js
        path to a synthetic-network scrypt. Default is a demo scripts/vendor/synthetic-network/frontend/udp_rate_sine_demo.js
        from the synthetic-network repo. Please consult synthetic-network repo for details: https://github.com/daily-co/synthetic-network
    --no-update
        skip git-submodule update for the synthetic-network repository
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --script-path)
            SCRIPT_PATH="$2"
            shift;shift
            ;;
        --no-update)
            UPDATE=false
            shift
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

SCRIPT_PATH=${SCRIPT_PATH:-scripts/vendor/synthetic-network/frontend/udp_rate_sine_demo.js}
SCRIPT_PATH=$(realpath $SCRIPT_PATH)
UPDATE=${UPDATE:-true}

if [[ "$UPDATE" = true ]]; then
    git submodule init
    git submodule update
fi

cd scripts/synthetic-network/vendor/synthetic-network/frontend

log "running .js script"
node $SCRIPT_PATH ${@:1}

exit 0
