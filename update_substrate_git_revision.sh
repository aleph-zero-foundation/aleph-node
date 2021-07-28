#!/bin/bash

set -e

rev=$(git ls-remote https://github.com/paritytech/substrate.git | grep refs/heads/master | cut -c 1-7)


paths=(bin/node/Cargo.toml bin/runtime/Cargo.toml finality-aleph/Cargo.toml
    primitives/Cargo.toml pallet/Cargo.toml)

for path in ${paths[@]}; do
    echo upgrade $path
    sed -e 's/\(substrate.git.*rev = "\).*"/\1'$rev'"/' < $path > x
    mv x $path
done

cargo update
