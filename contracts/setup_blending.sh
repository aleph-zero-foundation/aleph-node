#!/usr/bin/env bash

set -euo pipefail

SURI=//Alice
ROOT_DIR=$(pwd)

TOKENS=1000
VK_BYTES=0x00000000

INSTANTIATE_CMD="cargo contract instantiate --skip-confirm --suri ${SURI}"
CALL_CMD="cargo contract call --quiet --skip-confirm --suri ${SURI}"

TOKEN_ADDRESS=""
BLENDER_ADDRESS=""

function get_timestamp() {
 echo "$(date +'%Y-%m-%d %H:%M:%S')"
}

function error() {
   echo -e "[$(get_timestamp)] [ERROR] $*"
   exit 1
}

log_progress() {
  bold=$(tput bold)
  normal=$(tput sgr0)
  echo "[$(get_timestamp)] [INFO] ${bold}${1}${normal}"
}

run_chain() {
  cd "$ROOT_DIR"/../
  ./scripts/run_nodes.sh -b false 1> /dev/null 2> /dev/null
}

build_token_contract() {
  cd "$ROOT_DIR"/public_token/
  cargo contract build --quiet --release 1> /dev/null 2> /dev/null
}

deploy_token_contract() {
  cd "$ROOT_DIR"/public_token/
  result=$($INSTANTIATE_CMD --args ${TOKENS})
  TOKEN_ADDRESS=$(echo "$result" | grep Contract | tail -1 | cut -c 14-)
  echo "Token address: ${TOKEN_ADDRESS}"
}

set_allowance() {
  cd "$ROOT_DIR"/public_token/
  $CALL_CMD --contract ${TOKEN_ADDRESS} --message "PSP22::approve" --args ${BLENDER_ADDRESS} ${TOKENS} | grep "Success"
}

build_blender_contract() {
  cd "$ROOT_DIR"/blender/
  cargo contract build --quiet --release 1> /dev/null 2> /dev/null
}

deploy_blender_contract() {
  cd "$ROOT_DIR"/blender/
  result=$($INSTANTIATE_CMD)
  BLENDER_ADDRESS=$(echo "$result" | grep Contract | tail -1 | cut -c 14-)
  echo "Blender address: ${BLENDER_ADDRESS}"
}

register_vk() {
  cd "$ROOT_DIR"/blender/
  $CALL_CMD --contract ${BLENDER_ADDRESS} --message "register_vk" --args Deposit ${VK_BYTES} | grep "Success"
}

register_token() {
  cd "$ROOT_DIR"/blender/
  $CALL_CMD --contract ${BLENDER_ADDRESS} --message "register_new_token" --args 0 ${TOKEN_ADDRESS} | grep "Success"
}

log_progress "Launching local chain..."
run_chain || error "Failed to launch chain"

log_progress "Building token contract..."
build_token_contract || error "Failed to build token contract"

log_progress "Deploying token contract..."
deploy_token_contract || error "Failed to deploy token contract"

log_progress "Building blender contract..."
build_blender_contract || error "Failed to build blender contract"

log_progress "Deploying blender contract..."
deploy_blender_contract || error "Failed to deploy blender contract"

log_progress "Setting allowance for Blender..."
set_allowance || error "Failed to set allowance"

log_progress "Registering verifying key..."
register_vk || error "Failed to register verifying key"

log_progress "Registering token..."
register_token || error "Failed to register token"
