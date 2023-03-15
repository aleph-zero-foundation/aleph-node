#!/bin/bash

# To run this:
#
# .github/scripts/run_consensus.sh
# source contracts/env/dev
# contracts/scripts/deploy.sh
# contracts/scripts/test.sh

set -euox pipefail

# Move to the root of the repo
cd "$(dirname -- "$(readlink -f -- "${BASH_SOURCE[@]}")")/../.."

export CONTRACTS_PATH
CONTRACTS_PATH=$(pwd)/contracts

docker run \
  -e EARLY_BIRD_SPECIAL="$(jq --raw-output ".early_bird_special" < "$CONTRACTS_PATH"/addresses.json)" \
  -e THE_PRESSIAH_COMETH="$(jq --raw-output ".the_pressiah_cometh" < "$CONTRACTS_PATH"/addresses.json)" \
  -e BACK_TO_THE_FUTURE="$(jq --raw-output ".back_to_the_future" < "$CONTRACTS_PATH"/addresses.json)" \
  -e SIMPLE_DEX="$(jq --raw-output ".simple_dex" < "$CONTRACTS_PATH"/addresses.json)" \
  -e WRAPPED_AZERO="$(jq --raw-output ".wrapped_azero" < "$CONTRACTS_PATH"/addresses.json)" \
  -e BUTTON_GAME_METADATA="/code/contracts/button/target/ink/button.json" \
  -e TICKET_TOKEN_METADATA="/code/contracts/ticket_token/target/ink/ticket_token.json" \
  -e REWARD_TOKEN_METADATA="/code/contracts/game_token/target/ink/game_token.json" \
  -e MARKETPLACE_METADATA="/code/contracts/marketplace/target/ink/marketplace.json" \
  -e SIMPLE_DEX_METADATA="/code/contracts/simple_dex/target/ink/simple_dex.json" \
  -e WRAPPED_AZERO_METADATA="/code/contracts/wrapped_azero/target/ink/wrapped_azero.json" \
  -e RUST_LOG="aleph_e2e_client=info" \
  -v "$(pwd)":/code \
  -v ~/.cargo/registry:/usr/local/cargo/registry \
  -v ~/.cargo/git:/usr/local/cargo/git \
  -w /code/e2e-tests \
  --rm \
  --network host \
  rust:1.67-buster \
  cargo test button -- --test-threads 1 --nocapture
