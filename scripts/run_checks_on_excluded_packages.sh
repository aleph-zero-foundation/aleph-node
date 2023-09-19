#!/bin/bash

# set -x
set -eo pipefail

TOML_FILE="Cargo.toml"

# Read the TOML file and extract the `exclude` entries
packages=$(awk -F ' *= *' '/^exclude *= *\[/ {found=1} found && /^\]$/ {found=0} found' "$TOML_FILE")

packages="$(echo ${packages} | sed 's/[][,]/ /g' | sed 's/\s\+/\n/g' | sed '/^$/d')"

# Remove leading and trailing whitespace, and quotes from the entries
packages=$(echo "$packages" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e 's/^"//' -e 's/"$//')

packages="${packages//'%0A'/$'\n'}"

# Remove the key
packages=${packages:10}

for p in ${packages[@]}; do

  echo "Checking package $p ..."
  pushd "$p"

  if [[ "$p" =~ .*contracts.* ]] && [[ "$p" != "contracts/poseidon_host_bench" ]]; then
    echo "Disabling contract check as per https://github.com/727-Ventures/openbrush-contracts"
    echo "is not available."
    # cargo contract check
  elif [ "$p" = "baby-liminal-extension" ] || [ "$p" = "contracts/poseidon_host_bench" ]; then
    # cargo clippy --release --no-default-features --features substrate \
      #  --target wasm32-unknown-unknown -- --no-deps -D warnings
    :
  elif [ "$p" = "pallets/baby-liminal" ]; then
    cargo test --features runtime-benchmarks
  else
    cargo clippy -- --no-deps -D warnings
  fi

  popd

done
