#!/bin/bash

set -euo pipefail

# --- FUNCTIONS

source $(pwd)/.github/scripts/assert.sh

function link_bytecode() {
  local CONTRACT=$1
  local PLACEHOLDER=$2
  local REPLACEMENT=$3

  sed -i 's/'$PLACEHOLDER'/'$REPLACEMENT'/' target/ink/$CONTRACT.contract
}

# --- GLOBAL CONSTANTS
NODE_IMAGE=public.ecr.aws/p6e8q1z1/aleph-node:latest

NODE=ws://127.0.0.1:9943

ALICE=5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
ALICE_SEED=//Alice

NODE0=5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH
NODE0_SEED=//0

LIFETIME=5
TOTAL_BALANCE=1000
GAME_BALANCE=900

CONTRACTS_PATH=$(pwd)/contracts

## --- COMPILE CONTRACTS

cd $CONTRACTS_PATH/access_control
cargo contract build --release

cd $CONTRACTS_PATH/button_token
cargo contract build --release

cd $CONTRACTS_PATH/yellow_button
cargo contract build --release

## --- DEPLOY ACCESS CONTROL CONTRACT

cd $CONTRACTS_PATH/access_control

CONTRACT=$(cargo contract instantiate --url $NODE --constructor new --suri $ALICE_SEED)
ACCESS_CONTROL=$(echo "$CONTRACT" | grep Contract | tail -1 | cut -c 15-)
ACCESS_CONTROL_PUBKEY=$(docker run --rm --entrypoint "/bin/sh" "${NODE_IMAGE}" -c "aleph-node key inspect $ACCESS_CONTROL" | grep hex | cut -c 23- | cut -c 3-)

echo "access control contract address: " $ACCESS_CONTROL
echo "access control contract public key (hex): " $ACCESS_CONTROL_PUBKEY

## --- UPLOAD TOKEN CONTRACT

cd $CONTRACTS_PATH/button_token
# replace address placeholder with the on-chain address of the AccessControl contract
link_bytecode button_token 4465614444656144446561444465614444656144446561444465614444656144 $ACCESS_CONTROL_PUBKEY
# remove just in case
rm target/ink/button_token.wasm
# NOTE : here we go from hex to binary using a nodejs cli tool
# availiable from https://github.com/fbielejec/polkadot-cljs
node ../scripts/hex-to-wasm.js target/ink/button_token.contract target/ink/button_token.wasm

CODE_HASH=$(cargo contract upload --url $NODE --suri $ALICE_SEED)
BUTTON_TOKEN_CODE_HASH=$(echo "$CODE_HASH" | grep hash | tail -1 | cut -c 15-)

echo "button token code hash" $BUTTON_TOKEN_CODE_HASH

## --- GRANT INIT PRIVILEDGES ON THE TOKEN CONTRACT

cd $CONTRACTS_PATH/access_control

# alice is the initializer of the button-token contract
cargo contract call --url $NODE --contract $ACCESS_CONTROL --message grant_role --args $ALICE 'Initializer('$BUTTON_TOKEN_CODE_HASH')' --suri $ALICE_SEED

## --- INITIALIZE BUTTON TOKEN CONTRACT

cd $CONTRACTS_PATH/button_token

CONTRACT=$(cargo contract instantiate --url $NODE --constructor new --args $TOTAL_BALANCE --suri $ALICE_SEED)
BUTTON_TOKEN=$(echo "$CONTRACT" | grep Contract | tail -1 | cut -c 15-)

echo "button token contract instance address" $BUTTON_TOKEN

## --- GRANT PRIVILEDGES ON THE TOKEN CONTRACT

cd $CONTRACTS_PATH/access_control

# alice is the admin and the owner of the button-token contract
cargo contract call --url $NODE --contract $ACCESS_CONTROL --message grant_role --args $ALICE 'Admin('$BUTTON_TOKEN')' --suri $ALICE_SEED
cargo contract call --url $NODE --contract $ACCESS_CONTROL --message grant_role --args $ALICE 'Owner('$BUTTON_TOKEN')' --suri $ALICE_SEED

## --- UPLOAD GAME CONTRACT

cd $CONTRACTS_PATH/yellow_button
link_bytecode yellow_button 4465614444656144446561444465614444656144446561444465614444656144 $ACCESS_CONTROL_PUBKEY
rm target/ink/yellow_button.wasm
node ../scripts/hex-to-wasm.js target/ink/yellow_button.contract target/ink/yellow_button.wasm

CODE_HASH=$(cargo contract upload --url $NODE --suri $ALICE_SEED)
YELLOW_BUTTON_CODE_HASH=$(echo "$CODE_HASH" | grep hash | tail -1 | cut -c 15-)

echo "yellow button code hash" $YELLOW_BUTTON_CODE_HASH

## --- GRANT INIT PRIVILEDGES ON THE YELLOW BUTTON CONTRACT

cd $CONTRACTS_PATH/access_control

cargo contract call --url $NODE --contract $ACCESS_CONTROL --message grant_role --args $ALICE 'Initializer('$YELLOW_BUTTON_CODE_HASH')' --suri $ALICE_SEED

## --- INITIALIZE GAME CONTRACT

cd $CONTRACTS_PATH/yellow_button

CONTRACT=$(cargo contract instantiate --url $NODE --constructor new --args $BUTTON_TOKEN $LIFETIME --suri $ALICE_SEED)
YELLOW_BUTTON=$(echo "$CONTRACT" | grep Contract | tail -1 | cut -c 15-)

echo "yellow button contract instance address" $BUTTON_TOKEN

## --- GRANT PRIVILEDGES ON THE GAME CONTRACT

cd $CONTRACTS_PATH/access_control

cargo contract call --url $NODE --contract $ACCESS_CONTROL --message grant_role --args $ALICE 'Owner('$YELLOW_BUTTON')' --suri $ALICE_SEED
cargo contract call --url $NODE --contract $ACCESS_CONTROL --message grant_role --args $ALICE 'Admin('$YELLOW_BUTTON')' --suri $ALICE_SEED

## --- TRANSFER BALANCE TO THE GAME CONTRACT

cd $CONTRACTS_PATH/button_token

cargo contract call --url $NODE --contract $BUTTON_TOKEN --message transfer --args $YELLOW_BUTTON $GAME_BALANCE --suri $ALICE_SEED

## --- WHITELIST ACCOUNTS

cd $CONTRACTS_PATH/yellow_button

cargo contract call --url $NODE --contract $YELLOW_BUTTON --message bulk_allow --args "[$ALICE,$NODE0]" --suri $ALICE_SEED

## --- PLAY

cd $CONTRACTS_PATH/yellow_button

cargo contract call --url $NODE --contract $YELLOW_BUTTON --message press --suri $ALICE_SEED

sleep 1

cargo contract call --url $NODE --contract $YELLOW_BUTTON --message press --suri $NODE0_SEED

## --- TRIGGER DEATH AND REWARDS DISTRIBUTION

cd $CONTRACTS_PATH/yellow_button

sleep $(($LIFETIME + 1))

EVENT=$(cargo contract call --url $NODE --contract $YELLOW_BUTTON --message press --suri $ALICE_SEED | grep ButtonDeath)
EVENT=$(echo "$EVENT" | sed 's/^ *//g' | tr " " "\n")

PRESSIAH_REWARD=$(echo "$EVENT" | sed -n '7p' | tail -1)
PRESSIAH_REWARD=${PRESSIAH_REWARD::-1}

echo "The Pressiah receives: $PRESSIAH_REWARD"
assert_eq "450" "$PRESSIAH_REWARD"

exit $?
