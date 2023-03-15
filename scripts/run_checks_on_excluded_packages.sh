#!/bin/bash

set -e

# `packages` should reflect `exclude` section from root `Cargo.toml`.
packages=(
  "flooder"
  "e2e-tests"
  "aleph-client"
  "fork-off"
  "benches/payout-stakers"
  "bin/cliain"
  "contracts/access_control"
  "contracts/button"
  "contracts/game_token"
  "contracts/marketplace"
  "contracts/simple_dex"
  "contracts/ticket_token"
  "contracts/wrapped_azero"
  "contracts/adder, "
)

for p in ${packages[@]}
do
  echo "Checking package $p..."
  pushd "$p"
  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- --no-deps -D warnings
  popd
done
