#!/usr/bin/env bash

set -euo pipefail

function usage {
    cat << EOF
Usage:
  $0
    -t
      Test cases to run.
    -m
      Minimum number of validators before chain enters emergency state. Set in chain spec, stored as
      MinimumValidatorCount in pallet_Staking.
    -r
      Whether to randomize test case params, "true" and "false" values supported.
      Can only be used if both the `-f` and `-n` params are empty.
    -f
      Number of reserved seats available to validators, ignored if empty or `-n` is empty.
      Cannot be used with `-r=true`.
    -n
      Number of non-reserved seats available to validators, ignored if empty or `-f` is empty.
      Cannot be used with `-r=true`.
EOF
  exit 0
}

while getopts "h:t:m:r:f:n:" flag
do
  case "${flag}" in
    h) usage;;
    t) TEST_CASES="${OPTARG}";;
    m) MIN_VALIDATOR_COUNT="${OPTARG}";;
    r) RANDOMIZED="${OPTARG}";;
    f) RESERVED_SEATS="${OPTARG}";;
    n) NON_RESERVED_SEATS="${OPTARG}";;
    *)
      echo "Unrecognized argument "${flag}"!"
      usage
      exit 1
      ;;
  esac
done

if [[ -z "${MIN_VALIDATOR_COUNT:-}" ]]; then
  echo "Minimum validator count required but not provided!"
  usage
  exit 1
fi

# This is arbitrary.
MAX_VALIDATOR_COUNT=20

function set_randomized_test_params {
  VALIDATOR_COUNT=$(shuf -i "${MIN_VALIDATOR_COUNT}"-"${MAX_VALIDATOR_COUNT}" -n 1)
  # Assumes there is at least one reserved seat for validators.
  RESERVED_SEATS=$(shuf -i 1-"${VALIDATOR_COUNT}" -n 1)
  NON_RESERVED_SEATS=$((${VALIDATOR_COUNT} - ${RESERVED_SEATS}))
}

NODE_URL=${NODE_URL:-"ws://127.0.0.1:9943"}
NETWORK=${NETWORK:-"container:Node0"}

ARGS=(
  --network "${NETWORK}"
  -e NODE_URL="${NODE_URL}"
  -e RUST_LOG=info
  -e VALIDATOR_COUNT
)

if [[ -n "${TEST_CASES:-}" ]]; then
  ARGS+=(-e TEST_CASES="${TEST_CASES}")
fi

RANDOMIZED="${RANDOMIZED:-"false"}"
RESERVED_SEATS="${RESERVED_SEATS:-}"
NON_RESERVED_SEATS="${NON_RESERVED_SEATS:-}"

# Do not accept randomization together with test case parameters.
if [[ "${RANDOMIZED}" == "true" && ( -n "${RESERVED_SEATS}" || -n "${NON_RESERVED_SEATS}" )]]; then
  echo "Cannot both randomize and provide test case parameters!"
  exit 1
fi

# If randomization requested, generate random test params.
if [[ "${RANDOMIZED}" == "true" ]]; then
  set_randomized_test_params
  echo "Test case params have been randomized."
elif [[ "${RANDOMIZED}" == "false" ]]; then
  echo "Test case params have not been randomized."
else
  echo "Only 'true' and 'false' values supported, "${RANDOMIZED}" provided!"
  exit 1
fi

# If both test params are not empty, pass them. Otherwise, do not pass them.
if [[ -n "${RESERVED_SEATS}" && -n "${NON_RESERVED_SEATS}" ]]; then
  echo "Test case params: "${RESERVED_SEATS}" reserved and "${NON_RESERVED_SEATS}" non-reserved seats."
  ARGS+=(
    -e "${RESERVED_SEATS}"
    -e "${NON_RESERVED_SEATS}"
  )
else
  echo "Falling back on default test case param values."
fi

if [[ -n "${UPGRADE_VERSION:-}" && -n "${UPGRADE_SESSION:-}" && -n "${UPGRADE_FINALIZATION_WAIT_SESSIONS:-}" ]]; then
    ARGS+=(
        -e UPGRADE_VERSION
        -e UPGRADE_SESSION
        -e UPGRADE_FINALIZATION_WAIT_SESSIONS
    )
fi

if [[ -n "${ONLY_LEGACY:-}" ]]; then
    ARGS+=(-e ONLY_LEGACY)
fi

if [[ -n "${ADDER:-}" ]]; then
    ARGS+=(-e "ADDER=${ADDER}")
    ARGS+=(-e "ADDER_METADATA=/contracts/adder/target/ink/adder.json")
fi

if [[ -n "${BUTTON_GAME_METADATA:-}" ]]; then
    ARGS+=(-e "THE_PRESSIAH_COMETH=${THE_PRESSIAH_COMETH}")
    ARGS+=(-e "EARLY_BIRD_SPECIAL=${EARLY_BIRD_SPECIAL}")
    ARGS+=(-e "BACK_TO_THE_FUTURE=${BACK_TO_THE_FUTURE}")
    ARGS+=(-e "SIMPLE_DEX=${SIMPLE_DEX}")
    ARGS+=(-e "WRAPPED_AZERO=${WRAPPED_AZERO}")
    ARGS+=(-e "RUST_LOG=${RUST_LOG}")
    ARGS+=(-e "BUTTON_GAME_METADATA=/contracts/button/target/ink/button.json")
    ARGS+=(-e "TICKET_TOKEN_METADATA=/contracts/ticket_token/target/ink/ticket_token.json")
    ARGS+=(-e "REWARD_TOKEN_METADATA=/contracts/game_token/target/ink/game_token.json")
    ARGS+=(-e "MARKETPLACE_METADATA=/contracts/marketplace/target/ink/marketplace.json")
    ARGS+=(-e "SIMPLE_DEX_METADATA=/contracts/simple_dex/target/ink/simple_dex.json")
    ARGS+=(-e "WRAPPED_AZERO_METADATA=/contracts/wrapped_azero/target/ink/wrapped_azero.json")
fi

if [[ -n "${OUT_LATENCY:-}" ]]; then
    ARGS+=(-e OUT_LATENCY)
fi

docker run -v "$(pwd)/contracts:/contracts" -v "$(pwd)/docker/data:/data" "${ARGS[@]}" aleph-e2e-client:latest

exit $?
