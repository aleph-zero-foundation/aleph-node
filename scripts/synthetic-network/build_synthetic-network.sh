#!/bin/env bash

set -euo pipefail

source ./scripts/common.sh

UPDATE=${UPDATE:-true}

git submodule update --init
if [[ "$UPDATE" = true ]]; then
    git submodule update --remote
fi

pushd .
cd scripts/synthetic-network/vendor/synthetic-network

# this is a dirty-fix for the outdated version of node image used by the
# synthetic-network's Dockerfile
# TODO remove it after this push-request is merged into synthetic-network: https://github.com/daily-co/synthetic-network/pull/14
docker pull node:20.3.0
docker tag node:20.3.0 node:12.20.2

log "building base docker image for synthetic-network with support for synthetic-network"
docker build --tag syntheticnet --file Dockerfile .

popd

log "building docker image for aleph-node that supports synthetic-network"
docker build -t aleph-node:syntheticnet -f docker/Dockerfile.synthetic_network .

# clean previously tagged image
docker image rm node:12.20.2

exit 0
