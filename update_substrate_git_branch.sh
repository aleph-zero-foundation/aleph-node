#!/bin/bash

set -e

branch="$1"

paths=(bin/node/Cargo.toml bin/runtime/Cargo.toml finality-aleph/Cargo.toml
    primitives/Cargo.toml pallet/Cargo.toml)

for path in ${paths[@]}; do
    echo upgrade $path
    sed -e 's/\(substrate.git.*branch = "\)[^"]*".*\([,}]\)/\1'$branch'" \2/' < $path > x
    mv x $path
done

cargo update
