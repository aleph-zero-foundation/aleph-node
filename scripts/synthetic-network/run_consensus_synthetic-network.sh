#!/bin/env bash

set -euo pipefail

source ./scripts/common.sh

function usage(){
    cat << EOF
Usage:
  $0
  This script allows you to run aleph-node within docker and simulate some custom network conditions, e.g. delays, rate limit,
  package loss. Additionally, each node is preinstalled with the 'stress' tool, that allows to simulate high occupancy of nodes
  cpu and io. It should allow us test more realistic high volume network conditions without the need to spawn hundreds of
  aws instances. For more details on networking part of this solution, visit https://github.com/daily-co/synthetic-network .
  IMPORTANT: this script requires aleph-node:latest docker image.
    --no-build-image
        skip docker image build
    --commit 72bbb4fde915e4132c19cd7ce3605364abac58a5
        commit hash used to build synthetic-network, default is 72bbb4fde915e4132c19cd7ce3605364abac58a5
EOF
    exit 0
}

function build_test_image() {
    local commit=$1
    local path=$2

    GIT_COMMIT=$commit ${path}/build_synthetic-network.sh
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-build-image)
            BUILD_IMAGE=false
            shift
            ;;
        --commit)
            GIT_COMMIT="$2"
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

BUILD_IMAGE=${BUILD_IMAGE:-true}
GIT_COMMIT=${GIT_COMMIT:-72bbb4fde915e4132c19cd7ce3605364abac58a5}

if [[ "$BUILD_IMAGE" = true ]]; then
    log "building custom docker image for synthetic-network tests"
    path=$(dirname $0)
    build_test_image $GIT_COMMIT $path
fi

log "running synthetic-network"
DOCKER_COMPOSE=./docker/docker-compose.synthetic-network.yml ./.github/scripts/run_consensus.sh
log "open a web browser at http://localhost:3000 (port 3000 is Node0, 3001 is Node1, ...)"
xdg-open http://localhost:3000

exit 0
