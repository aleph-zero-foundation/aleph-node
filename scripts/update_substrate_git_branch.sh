#!/bin/bash

set -e

branch="$1"

# Find all `Cargo.toml` files outside any `target` directory.
paths=$(find . -mindepth 2 -type f -name "Cargo.toml" -not -path "*/target/*")

for path in ${paths[@]}; do
    echo upgrade $path
    # 1. Find and capture `Cardinal-Cryptography/substrate.git", branch = "` substring. It will be available as `\1`. In place
    #    of spaces there can be sequence of `\s` characters.
    # 2. Find and capture whatever is after closing `"` and before `,` or `}`. It will be available as `\2`.
    # 3. Substitute new branch and concatenate it with `\1` and `\2`.
    sed -e 's/\(Cardinal-Cryptography\/substrate.git"\s*,\s*branch\s*=\s*"\)[^"]*"\([^,}]*\)/\1'$branch'"\2/' < $path > x
    mv x $path
done

cargo update
