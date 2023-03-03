#!/bin/env bash

set -euo pipefail

source ./scripts/common.sh

UPDATE=${UPDATE:-true}

if [[ "$UPDATE" = true ]]; then
    git submodule init
    git submodule update
fi

pushd .
cd scripts/synthetic-network/vendor/synthetic-network

log "building base docker image for synthetic-network with support for synthetic-network"
docker build -t syntheticnet .

popd

log "building docker image for aleph-node that supports synthetic-network"
docker build -t aleph-node:syntheticnet -f docker/Dockerfile.synthetic_network .

exit 0
