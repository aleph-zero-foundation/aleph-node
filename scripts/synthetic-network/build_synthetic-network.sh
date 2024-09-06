#!/usr/bin/env bash

set -euo pipefail

source ./scripts/common.sh

UPDATE=${UPDATE:-true}

git submodule update --init
if [[ "$UPDATE" = true ]]; then
    git submodule update --remote
fi

log "building docker image for synthetic-network"
docker build --tag syntheticnet --file docker/Dockerfile.synthetic_network.build scripts/synthetic-network/vendor/synthetic-network

log "building docker image for aleph-node that supports synthetic-network"
docker build --tag aleph-node:syntheticnet --file docker/Dockerfile.synthetic_network .

exit 0
