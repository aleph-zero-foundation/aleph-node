#!/bin/env bash

set -euo pipefail

source ./scripts/common.sh

GIT_COMMIT=${GIT_COMMIT:-72bbb4fde915e4132c19cd7ce3605364abac58a5}

TMPDIR="$(dirname $0)/vendor"
mkdir -p $TMPDIR
log "created a temporary folder at $TMPDIR"

pushd .

log "cloning synthetic-network's git repo"
cd $TMPDIR
if [[ ! -d ./synthetic-network ]]; then
    git clone https://github.com/daily-co/synthetic-network.git
fi
cd synthetic-network
git fetch origin
git checkout $GIT_COMMIT

log "building base docker image for synthetic-network with support for synthetic-network"
log "patching synthetic network"
# aleph-node crashes since it uses newer glibc than this image
sed -i 's/FROM node:12.20.2/FROM node:19.2/' Dockerfile
docker build -t syntheticnet .

popd

log "building docker image for aleph-node that supports synthetic-network"
docker build -t aleph-node:syntheticnet -f docker/Dockerfile.synthetic_network .

exit 0
