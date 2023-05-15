#!/bin/bash

set -euox pipefail

# --- GLOBAL CONSTANTS
NODE_IMAGE=public.ecr.aws/p6e8q1z1/aleph-node:latest

CONTRACTS_PATH=$(pwd)/contracts

# --- FUNCTIONS

function run_ink_builder() {
  docker start ink_builder || docker run \
    --network host \
    -v "${PWD}:/code" \
    -u "$(id -u):$(id -g)" \
    --name ink_builder \
    --platform linux/amd64 \
    --detach \
    --rm public.ecr.aws/p6e8q1z1/ink-dev:1.0.0 sleep 1d
}

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

function upload_contract {

  local  __resultvar=$1
  local contract_name=$2

  cd "$CONTRACTS_PATH"/$contract_name

  # --- REPLACE THE ADDRESS OF ACCESS CONTROL CONTRACT

  # replace address placeholder with the on-chain address of the AccessControl contract
  link_bytecode $contract_name 4465614444656144446561444465614444656144446561444465614444656144 $ACCESS_CONTROL_PUBKEY
  # remove just in case
  rm target/ink/$contract_name.wasm
  # NOTE : here we go from hex to binary using a nodejs cli tool
  # availiable from https://github.com/fbielejec/polkadot-cljs
  node ../scripts/hex-to-wasm.js target/ink/$contract_name.contract target/ink/$contract_name.wasm

  # --- UPLOAD CONTRACT CODE

  code_hash=$(cargo_contract upload --url "$NODE" --suri "$AUTHORITY_SEED" --output-json | jq -r '.code_hash')

  echo "$contract_name code hash: $code_hash"

  cd "$CONTRACTS_PATH"/access_control

  # Set the initializer of the contract
  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Initializer('$code_hash')' --suri "$AUTHORITY_SEED" --skip-confirm

  eval $__resultvar="'$code_hash'"
}

function deploy_ticket_token {

  local  __resultvar=$1
  local token_name=$2
  local token_symbol=$3
  local salt=$4

  # --- CREATE AN INSTANCE OF THE TICKET CONTRACT

  cd "$CONTRACTS_PATH"/ticket_token

  local contract_address=$(cargo_contract instantiate --url "$NODE" --constructor new --args \"$token_name\" \"$token_symbol\" "$TICKET_BALANCE" --suri "$AUTHORITY_SEED" --salt "$salt" --skip-confirm --output-json)
  local contract_address=$(echo "$contract_address" | jq -r '.contract')

  echo "$token_symbol ticket contract instance address:  $contract_address"

  # --- GRANT PRIVILEGES ON THE TICKET CONTRACT

  cd "$CONTRACTS_PATH"/access_control

  # set the admin of the contract instance
  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Admin('"$contract_address"')' --suri "$AUTHORITY_SEED" --skip-confirm

  eval $__resultvar="'$contract_address'"
}

function deploy_game_token {

  local  __resultvar=$1
  local token_name=$2
  local token_symbol=$3
  local salt=$4

  # --- CREATE AN INSTANCE OF THE TOKEN CONTRACT

  cd "$CONTRACTS_PATH"/game_token

  local contract_address=$(cargo_contract instantiate --url "$NODE" --constructor new --args \"$token_name\" \"$token_symbol\" --suri "$AUTHORITY_SEED" --salt "$salt" --skip-confirm --output-json)
  local contract_address=$(echo "$contract_address" | jq -r '.contract')

  echo "$token_symbol token contract instance address: $contract_address"

  # --- GRANT PRIVILEGES ON THE TOKEN CONTRACT

  cd "$CONTRACTS_PATH"/access_control

  # set the admin of the contract instance
  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Admin('"$contract_address"')' --suri "$AUTHORITY_SEED" --skip-confirm

  eval "$__resultvar='$contract_address'"
}

function deploy_button_game {

  local  __resultvar=$1
  local game_type=$2
  local ticket_token=$3
  local game_token=$4
  local marketplace=$5
  local salt=$6

  # --- CREATE AN INSTANCE OF THE CONTRACT

  cd "$CONTRACTS_PATH"/button

  local contract_address=$(cargo_contract instantiate --url "$NODE" --constructor new --args "$ticket_token" "$game_token" "$marketplace" "$LIFETIME" "$game_type" "$BUTTON_MIN_REWARD" "$BUTTON_MAX_REWARD" --suri "$AUTHORITY_SEED" --salt "$salt" --skip-confirm --output-json)
  local contract_address=$(echo "$contract_address" | jq -r '.contract')
  echo "$game_type contract instance address: $contract_address"

  # --- GRANT PRIVILEGES ON THE CONTRACT

  cd "$CONTRACTS_PATH"/access_control

  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Admin('"$contract_address"')' --suri "$AUTHORITY_SEED" --skip-confirm
  if [ "$ENV_NAME" = "dev" ]; then
    # grant minter role on the game token to the authority address
    cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Custom('"$game_token"',[0x4D,0x49,0x4E,0x54])' --suri "$AUTHORITY_SEED" --skip-confirm
  fi
  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$contract_address" 'Admin('"$marketplace"')' --suri "$AUTHORITY_SEED" --skip-confirm
  # grant minter role on the game token to the contract
  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$contract_address" 'Custom('"$game_token"',[0x4D,0x49,0x4E,0x54])' --suri "$AUTHORITY_SEED" --skip-confirm

  eval "$__resultvar='$contract_address'"
}

function deploy_marketplace {
  local  __resultvar=$1
  local code_hash=$2
  local contract_name=$3
  local salt=$4
  local ticket_token=$5
  local game_token=$6

  local sale_price_multiplier=2

  # --- CREATE AN INSTANCE OF THE CONTRACT

  cd "$CONTRACTS_PATH"/marketplace

  local contract_address
  contract_address=$(cargo_contract instantiate --url "$NODE" --constructor new \
    --args "$ticket_token" "$game_token" "$INITIAL_PRICE" "$MINIMAL_PRICE" "$sale_price_multiplier" "$AUCTION_LENGTH" \
    --suri "$AUTHORITY_SEED" --salt "$salt" --skip-confirm --output-json)
  contract_address=$(echo "$contract_address" | jq -r '.contract')

  echo "Marketplace for $contract_name instance address: $contract_address"

  # --- GRANT PRIVILEGES ON THE CONTRACT

  cd "$CONTRACTS_PATH"/access_control

  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Admin('"$contract_address"')' --suri "$AUTHORITY_SEED" --skip-confirm
  # grant burner role on game token to the contract
  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$contract_address" 'Custom('"$game_token"',[0x42,0x55,0x52,0x4E])' --suri "$AUTHORITY_SEED" --skip-confirm

  eval "$__resultvar='$contract_address'"
}

function deploy_simple_dex {
  local  __resultvar=$1

  # --- CREATE AN INSTANCE OF THE CONTRACT

  cd "$CONTRACTS_PATH"/simple_dex

  local contract_address
  contract_address=$(cargo_contract instantiate --url "$NODE" --constructor new --suri "$AUTHORITY_SEED" --skip-confirm --output-json)
  contract_address=$(echo "$contract_address" | jq -r '.contract')

  echo "Simple dex contract instance address: $contract_address"

  # --- GRANT PRIVILEGES ON THE CONTRACT

  cd "$CONTRACTS_PATH"/access_control

  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Admin('"$contract_address"')' --suri "$AUTHORITY_SEED" --skip-confirm
  # grant Liquidity Provider role to the authority
  cargo_contract call --url "$NODE" --contract "$ACCESS_CONTROL" --message grant_role --args "$AUTHORITY" 'Custom('"$contract_address"',[0x4C,0x51,0x54,0x59])' --suri "$AUTHORITY_SEED" --skip-confirm

  eval "$__resultvar='$contract_address'"
}

function whitelist_swap_pair() {
  local from_address=$1
  local to_address=$2

  cd "$CONTRACTS_PATH"/simple_dex

  cargo_contract call --url "$NODE" --contract "$SIMPLE_DEX" --message add_swap_pair --args "$from_address" "$to_address" --suri "$AUTHORITY_SEED" --skip-confirm
}

function deploy_wrapped_azero {
  local  __resultvar=$1

  # --- CREATE AN INSTANCE OF THE CONTRACT

  cd "$CONTRACTS_PATH"/wrapped_azero

  local contract_address
  contract_address=$(cargo_contract instantiate --url "$NODE" --constructor new --suri "$AUTHORITY_SEED" --skip-confirm --output-json)
  contract_address=$(echo "$contract_address" | jq -r '.contract')

  echo "wrapped Azero contract instance address: $contract_address"

  eval "$__resultvar='$contract_address'"
}

function link_bytecode() {
  local contract=$1
  local placeholder=$2
  local replacement=$3

  sed -i 's/'"$placeholder"'/'"$replacement"'/' "target/ink/$contract.contract"
}

# --- COMPILE CONTRACTS

run_ink_builder

cd "$CONTRACTS_PATH"/access_control
ink_build rustup target add wasm32-unknown-unknown
ink_build rustup component add rust-src
cargo_contract build --release

cd "$CONTRACTS_PATH"/ticket_token
cargo_contract build --release

cd "$CONTRACTS_PATH"/game_token
cargo_contract build --release

cd "$CONTRACTS_PATH"/button
cargo_contract build --release

cd "$CONTRACTS_PATH"/marketplace
cargo_contract build --release

cd "$CONTRACTS_PATH"/simple_dex
cargo_contract build --release

cd "$CONTRACTS_PATH"/wrapped_azero
if [ "${ENV_NAME}" = "devnet" ] || [ "${ENV_NAME}" = "dev" ]; then
  echo "Compiling wrapped_azero for devnet environments. This will include an unguarded terminate flag!"
  cargo_contract build --release --features devnet
else
  echo "Compiling wrapped_azero for production environments."
  cargo_contract build --release
fi

# # --- DEPLOY ACCESS CONTROL CONTRACT

cd "$CONTRACTS_PATH"/access_control

ACCESS_CONTROL_CODE_HASH=$(cargo_contract upload --url "$NODE" --suri "$AUTHORITY_SEED" --output-json | jq -s . | jq -r '.[1].code_hash')
ACCESS_CONTROL=$(cargo_contract instantiate --url "$NODE" --constructor new --suri "$AUTHORITY_SEED" --skip-confirm --output-json | jq -r '.contract')
ACCESS_CONTROL_PUBKEY=$(docker run --rm --entrypoint "/bin/sh" "${NODE_IMAGE}" -c "aleph-node key inspect $ACCESS_CONTROL" | grep hex | cut -c 23- | cut -c 3-)

echo "access control contract address: $ACCESS_CONTROL"
echo "access control contract public key \(hex\): $ACCESS_CONTROL_PUBKEY"

# --- UPLOAD CONTRACTS CODES

upload_contract TICKET_TOKEN_CODE_HASH ticket_token
upload_contract GAME_TOKEN_CODE_HASH game_token
upload_contract BUTTON_CODE_HASH button
upload_contract MARKETPLACE_CODE_HASH marketplace
upload_contract SIMPLE_DEX_CODE_HASH simple_dex
upload_contract WRAPPED_AZERO_CODE_HASH wrapped_azero

start=$(date +%s.%N)

#
# --- EARLY_BIRD_SPECIAL GAME
#
echo "Early Bird Special"

salt="0x4561726C79426972645370656369616C"
deploy_ticket_token EARLY_BIRD_SPECIAL_TICKET early_bird_special_ticket EBST $salt
deploy_game_token EARLY_BIRD_SPECIAL_TOKEN Ubik UBI $salt
deploy_marketplace EARLY_BIRD_SPECIAL_MARKETPLACE "$MARKETPLACE_CODE_HASH" early_bird_special "$salt" "$EARLY_BIRD_SPECIAL_TICKET" "$EARLY_BIRD_SPECIAL_TOKEN"
deploy_button_game EARLY_BIRD_SPECIAL EarlyBirdSpecial "$EARLY_BIRD_SPECIAL_TICKET" "$EARLY_BIRD_SPECIAL_TOKEN" "$EARLY_BIRD_SPECIAL_MARKETPLACE" "$salt"

#
# --- BACK_TO_THE_FUTURE GAME
#
echo "Back To The Future"

salt="0x4261636B546F546865467574757265"
deploy_ticket_token BACK_TO_THE_FUTURE_TICKET back_to_the_future_ticket BTFT $salt
deploy_game_token BACK_TO_THE_FUTURE_TOKEN Cyberiad CYB $salt
deploy_marketplace BACK_TO_THE_FUTURE_MARKETPLACE "$MARKETPLACE_CODE_HASH" back_to_the_future "$salt" "$BACK_TO_THE_FUTURE_TICKET" "$BACK_TO_THE_FUTURE_TOKEN"
deploy_button_game BACK_TO_THE_FUTURE BackToTheFuture "$BACK_TO_THE_FUTURE_TICKET" "$BACK_TO_THE_FUTURE_TOKEN" "$BACK_TO_THE_FUTURE_MARKETPLACE" "$salt"

#
# --- THE_PRESSIAH_COMETH GAME
#
echo "The Pressiah Cometh"

salt="0x7468655F70726573736961685F636F6D657468"
deploy_ticket_token THE_PRESSIAH_COMETH_TICKET the_pressiah_cometh_ticket TPCT $salt
deploy_game_token THE_PRESSIAH_COMETH_TOKEN Lono LON $salt
deploy_marketplace THE_PRESSIAH_COMETH_MARKETPLACE "$MARKETPLACE_CODE_HASH" the_pressiah_cometh "$salt" "$THE_PRESSIAH_COMETH_TICKET" "$THE_PRESSIAH_COMETH_TOKEN"
deploy_button_game THE_PRESSIAH_COMETH ThePressiahCometh "$THE_PRESSIAH_COMETH_TICKET" "$THE_PRESSIAH_COMETH_TOKEN" "$THE_PRESSIAH_COMETH_MARKETPLACE" "$salt"

# --- DEPLOY WRAPPED AZERO CONTRACT

echo "Wrapped Azero"
deploy_wrapped_azero WRAPPED_AZERO

# --- DEPLOY DEX CONTRACT

echo "Simple Dex"
deploy_simple_dex SIMPLE_DEX

echo "Whitelisting swap token pairs"
whitelist_swap_pair $EARLY_BIRD_SPECIAL_TOKEN $BACK_TO_THE_FUTURE_TOKEN
whitelist_swap_pair $EARLY_BIRD_SPECIAL_TOKEN $THE_PRESSIAH_COMETH_TOKEN
whitelist_swap_pair $EARLY_BIRD_SPECIAL_TOKEN $WRAPPED_AZERO

whitelist_swap_pair $BACK_TO_THE_FUTURE_TOKEN $EARLY_BIRD_SPECIAL_TOKEN
whitelist_swap_pair $BACK_TO_THE_FUTURE_TOKEN $THE_PRESSIAH_COMETH_TOKEN
whitelist_swap_pair $BACK_TO_THE_FUTURE_TOKEN $WRAPPED_AZERO

whitelist_swap_pair $THE_PRESSIAH_COMETH_TOKEN $EARLY_BIRD_SPECIAL_TOKEN
whitelist_swap_pair $THE_PRESSIAH_COMETH_TOKEN $BACK_TO_THE_FUTURE_TOKEN
whitelist_swap_pair $THE_PRESSIAH_COMETH_TOKEN $WRAPPED_AZERO

# spit adresses to a JSON file
cd "$CONTRACTS_PATH"

jq -n \
   --arg early_bird_special "$EARLY_BIRD_SPECIAL" \
   --arg early_bird_special_marketplace "$EARLY_BIRD_SPECIAL_MARKETPLACE" \
   --arg early_bird_special_ticket "$EARLY_BIRD_SPECIAL_TICKET" \
   --arg early_bird_special_token "$EARLY_BIRD_SPECIAL_TOKEN" \
   --arg back_to_the_future "$BACK_TO_THE_FUTURE" \
   --arg back_to_the_future_ticket "$BACK_TO_THE_FUTURE_TICKET" \
   --arg back_to_the_future_token "$BACK_TO_THE_FUTURE_TOKEN" \
   --arg back_to_the_future_marketplace "$BACK_TO_THE_FUTURE_MARKETPLACE" \
   --arg the_pressiah_cometh "$THE_PRESSIAH_COMETH" \
   --arg the_pressiah_cometh_ticket "$THE_PRESSIAH_COMETH_TICKET" \
   --arg the_pressiah_cometh_token "$THE_PRESSIAH_COMETH_TOKEN" \
   --arg the_pressiah_cometh_marketplace "$THE_PRESSIAH_COMETH_MARKETPLACE" \
   --arg button_code_hash "$BUTTON_CODE_HASH" \
   --arg ticket_token_code_hash "$TICKET_TOKEN_CODE_HASH" \
   --arg game_token_code_hash "$GAME_TOKEN_CODE_HASH" \
   --arg marketplace_code_hash "$MARKETPLACE_CODE_HASH" \
   --arg access_control "$ACCESS_CONTROL" \
   --arg access_control_code_hash "$ACCESS_CONTROL_CODE_HASH" \
   --arg simple_dex "$SIMPLE_DEX" \
   --arg simple_dex_code_hash "$SIMPLE_DEX_CODE_HASH" \
   --arg wrapped_azero "$WRAPPED_AZERO" \
   --arg wrapped_azero_code_hash "$WRAPPED_AZERO_CODE_HASH" \
   '{
      early_bird_special: $early_bird_special,
      early_bird_special_marketplace: $early_bird_special_marketplace,
      early_bird_special_ticket: $early_bird_special_ticket,
      early_bird_special_token: $early_bird_special_token,
      back_to_the_future: $back_to_the_future,
      back_to_the_future_ticket: $back_to_the_future_ticket,
      back_to_the_future_token: $back_to_the_future_token,
      back_to_the_future_marketplace: $back_to_the_future_marketplace,
      the_pressiah_cometh: $the_pressiah_cometh,
      the_pressiah_cometh_ticket: $the_pressiah_cometh_ticket,
      the_pressiah_cometh_token: $the_pressiah_cometh_token,
      the_pressiah_cometh_marketplace: $the_pressiah_cometh_marketplace,
      access_control: $access_control,
      simple_dex: $simple_dex,
      wrapped_azero: $wrapped_azero,
      button_code_hash: $button_code_hash,
      ticket_token_code_hash: $ticket_token_code_hash,
      game_token_code_hash: $game_token_code_hash,
      marketplace_code_hash: $marketplace_code_hash,
      access_control_code_hash: $access_control_code_hash,
      simple_dex_code_hash: $simple_dex_code_hash,
      wrapped_azero_code_hash: $wrapped_azero_code_hash
    }' > addresses.json

end=`date +%s.%N`
echo "Time elapsed: $( echo "$end - $start" | bc -l )"
cat addresses.json

exit $?
