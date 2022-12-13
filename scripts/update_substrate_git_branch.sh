#!/usr/bin/env bash

set -euo pipefail

function usage(){
  cat << EOF
Substitutes the branch name of the repository `https://github.com/Cardinal-Cryptography/substrate` in all Cargo.toml files.

Usage:
  $0 <new_branch_name>
EOF
  exit 0
}

BRANCH="${1:-}"
if [[ -z "${BRANCH}" ]]; then
       usage
       exit 2
fi

# Find all `Cargo.toml` files outside any `target` directory.
paths=$(find . -mindepth 2 -type f -name "Cargo.toml" -not -path "*/target/*") || echo "Problems with finding Cargo.toml files"

for path in ${paths[@]}; do
    echo "Upgrading ${path}"
    # 1. Filter out lines not containing `https://github.com/Cardinal-Cryptography/substrate[.git]"`.
    # 2. Substitute `###` in `branch = "###"` with $BRANCH.
    sed -e '/https:\/\/github.com\/Cardinal-Cryptography\/substrate\(.git\)\{0,1\}"/s/\(branch\s*=\s*"\)[^"]*"\([^,}]*\)/\1'"${BRANCH//\//\\/}"'"\2/' < $path > x
    mv x "${path}"

    cargo update --manifest-path "${path}"
done

exit 0
