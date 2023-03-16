#!/bin/bash

set -euo pipefail

# --- FUNCTIONS

function ink_build() {
  contract_dir=$(basename "${PWD}")

  docker exec \
    -u "$(id -u):$(id -g)" \
    -w "/code/contracts/$contract_dir" \
    ink_builder "$@"
}

function cargo_contract() {
  ink_build cargo contract "$@"
}

function terminate_contract {
  local contract_name=$1
  local contract_dir=$2
  local contract_address=$(get_address $contract_name)
  local tmp_output_file="tmp_terminate_contract_output"

  cd "$CONTRACTS_PATH"/"$contract_dir"
  # When 'ContractNotFound' occurs, ignore it and continue
  set +e
  cargo_contract call --url "$NODE" --contract $contract_address --message terminate --suri "$AUTHORITY_SEED" --skip-confirm 2>&1 | tee $tmp_output_file
  if [ $? -ne 0 ]; then
    grep -q "ContractNotFound" $tmp_output_file
    if [ $? -ne 0 ]; then
      echo "** Exiting with error code 1. Error is different than 'ContractNotFound'"
      exit 1
    else
      echo "** Ignoring 'ContractNotFound' error."
    fi
  fi
  rm -f $tmp_output_file
  set -e
}

function get_address {
  local contract_name=$1
  cat "$CONTRACTS_PATH"/addresses.json | jq --raw-output ".$contract_name"
}

function remove_contract_code {
  local code_hash=$(cat "$CONTRACTS_PATH"/addresses.json | jq --raw-output ".$1")
  if [ "None" == "$(docker run --network host -e RUST_LOG=info "$CLIAIN_IMAGE" --seed "$AUTHORITY_SEED" --node "$NODE" contract-owner-info --code-hash "$code_hash")" ]; then
    echo "Contract code does not exist on chain."
  else
    echo 'Contract code ' ${code_hash} 'exists, removing'
    docker run --network host -e RUST_LOG=info "$CLIAIN_IMAGE" --seed "$AUTHORITY_SEED" --node "$NODE" contract-remove-code --code-hash "$code_hash"
  fi
}

# --- GLOBAL CONSTANTS

CONTRACTS_PATH=$(pwd)/contracts
CLIAIN_IMAGE=public.ecr.aws/p6e8q1z1/cliain:latest

# --- CLEAN BUTTON CONTRACT

terminate_contract early_bird_special button
terminate_contract early_bird_special_marketplace marketplace
terminate_contract early_bird_special_ticket ticket_token
terminate_contract early_bird_special_token game_token
echo "succesfully terminated early_bird_special"

terminate_contract back_to_the_future button
terminate_contract back_to_the_future_ticket ticket_token
terminate_contract back_to_the_future_token game_token
terminate_contract back_to_the_future_marketplace marketplace
echo "succesfully terminated back_to_the_future"

terminate_contract the_pressiah_cometh button
terminate_contract the_pressiah_cometh_ticket ticket_token
terminate_contract the_pressiah_cometh_token game_token
terminate_contract the_pressiah_cometh_marketplace marketplace
echo "succesfully terminated the_pressiah_cometh"

remove_contract_code button_code_hash
remove_contract_code ticket_token_code_hash
remove_contract_code game_token_code_hash
remove_contract_code marketplace_code_hash
echo "succesfully removed code hashes"

terminate_contract simple_dex simple_dex
remove_contract_code simple_dex_code_hash
echo "succesfully terminated and removed simple_dex"

terminate_contract wrapped_azero wrapped_azero
remove_contract_code wrapped_azero_code_hash
echo "succesfully terminated and removed wrapped_azero"

# remove access control as last
terminate_contract access_control access_control
remove_contract_code access_control_code_hash
echo "succesfully terminated and removed AccessControl"

exit $?
