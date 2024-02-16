#!/usr/bin/env bash

set -euo pipefail
source ./scripts/common.sh

# ------------------------ constants -------------------------------------------

export NODE_ID=5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH
CHAINSPEC_FILE="./benchmark-chainspec.json"

# ------------------------ argument parsing and usage --------------------------

function usage(){
  cat << EOF
Usage:
  $0
  --feature-control
      Run benchmarks for the feature-control pallet
  --vk-storage
      Run benchmarks for the vk-storage pallet
  --chain-extension
      Run benchmarks for the baby liminal chain extension
EOF
  exit 0
}

VK_STORAGE=""
FEATURE_CONTROL=""
CHAIN_EXTENSION=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --feature-control)
      FEATURE_CONTROL="true"
      shift
      ;;
    --vk-storage)
      VK_STORAGE="true"
      shift
      ;;
    --chain-extension)
      CHAIN_EXTENSION="true"
      shift
      ;;
    --help)
      usage
      shift
      ;;
    *)
      error "Unrecognized argument $1!"
      ;;
  esac
done

# ------------------------ cleaning --------------------------------------------
function cleanup() {
  rm -rf "${CHAINSPEC_FILE}"
}

function sigint_trap() {
  echo "Ctrl+C pressed, performing cleanup."
  cleanup
}

trap sigint_trap SIGINT
trap cleanup EXIT

# ------------------------ functions -------------------------------------------
function bootstrap() {
  cargo run --profile production -p aleph-node --features runtime-benchmarks -- bootstrap-chain \
    --base-path /tmp/ \
    --account-ids $NODE_ID \
    --sudo-account-id $NODE_ID \
    --chain-id benchmarknet \
    --token-symbol BZERO \
    --chain-name 'Aleph Zero BenchmarkNet' \
    > "${CHAINSPEC_FILE}"
}

function benchmark() {
  local target="$1";
  local output_path="$2";

  cargo run --profile production -p aleph-node --features runtime-benchmarks -- benchmark pallet \
        --chain="${CHAINSPEC_FILE}" \
        --pallet="${target}" \
        --extrinsic='*' \
        --steps=20 \
        --repeat=5 \
        --template=.maintain/pallet-weight-template.hbs \
        --wasm-execution=compiled \
        --output="${output_path}"
}

function benchmark_vk_storage_pallet() {
  benchmark pallet_vk_storage pallets/vk-storage/src/weights.rs
}

function benchmark_feature_control_pallet() {
  benchmark pallet_feature_control pallets/feature-control/src/weights.rs
}

function benchmark_chain_extension() {
  benchmark baby_liminal_extension baby-liminal-extension/src/backend/weights.rs
}

# ------------------------ main ------------------------------------------------

if [[ -z "${FEATURE_CONTROL}" && -z "${VK_STORAGE}" && -z "${CHAIN_EXTENSION}" ]] ; then
  echo "No benchmarks selected, exiting."
fi

if [[ "${FEATURE_CONTROL}" == "true" ]]; then
  bootstrap
  benchmark_feature_control_pallet
fi

if [[ "${VK_STORAGE}" == "true" ]]; then
  bootstrap
  benchmark_vk_storage_pallet
fi

if [[ "${CHAIN_EXTENSION}" == "true" ]]; then
  bootstrap
  benchmark_chain_extension
fi

exit 0
