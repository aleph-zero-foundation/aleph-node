#!/usr/bin/env bash
set -euo pipefail

# load env variables from a file if ENV_FILE is set
if [[ -n "${ENV_FILE:-}" ]] && [[ -f "${ENV_FILE}" ]]; then
  set -o allexport
  source ${ENV_FILE}
  set +o allexport
fi

# script env variables
PURGE_BEFORE_START=${PURGE_BEFORE_START:-}
ALLOW_PRIVATE_IPV4=${ALLOW_PRIVATE_IPV4:-}
DISCOVER_LOCAL=${DISCOVER_LOCAL:-}

# aleph_node cli options to env variables
CHAIN=${CHAIN:?'Chain should be specified'}
NAME=${NAME:?'Name should be specified'}
BASE_PATH=${BASE_PATH:?'Base path should be specified'}
RPC_PORT=${RPC_PORT:-9933}
WS_PORT=${WS_PORT:-9943}
PORT=${PORT:-30333}
VALIDATOR_PORT=${VALIDATOR_PORT:-30343}
EXTERNAL_PORT=${EXTERNAL_PORT:-${PORT}}
VALIDATOR=${VALIDATOR:-true}
WS_MAX_CONNECTIONS=${WS_MAX_CONNECTIONS:-100}
POOL_LIMIT=${POOL_LIMIT:-1024}
PROMETHEUS_ENABLED=${PROMETHEUS_ENABLED:-true}
TELEMETRY_ENABLED=${TELEMETRY_ENABLED:-false}
TELEMETRY_URL=${TELEMETRY_URL:-'wss://telemetry.polkadot.io/submit/'}
TELEMETRY_VERBOSITY_LVL=${TELEMETRY_VERBOSITY_LVL:-'0'}
UNIT_CREATION_DELAY=${UNIT_CREATION_DELAY:-300}
DB_CACHE=${DB_CACHE:-1024}
RUNTIME_CACHE_SIZE=${RUNTIME_CACHE_SIZE:-2}
MAX_RUNTIME_INSTANCES=${MAX_RUNTIME_INSTANCES:-8}
BACKUP_PATH=${BACKUP_PATH:-${BASE_PATH}/backup-stash}

if [[ "true" == "$PURGE_BEFORE_START" ]]; then
  echo "Purging chain (${CHAIN}) at path ${BASE_PATH}"
  aleph-node purge-chain --base-path "${BASE_PATH}" --chain "${CHAIN}" -y
fi

ARGS=(
  --validator
  --execution Native
  --name "${NAME}"
  --base-path "${BASE_PATH}"
  --pool-limit "${POOL_LIMIT}"
  --chain "${CHAIN}"
  --node-key-file "${NODE_KEY_PATH}"
  --backup-path "${BACKUP_PATH}"
  --rpc-port "${RPC_PORT}"
  --ws-port "${WS_PORT}"
  --port "${PORT}"
  --validator-port "${VALIDATOR_PORT}"
  --rpc-cors all
  --no-mdns
  --ws-max-connections "${WS_MAX_CONNECTIONS}"
  --unsafe-ws-external --unsafe-rpc-external
  --enable-log-reloading
  --db-cache "${DB_CACHE}"
  --runtime-cache-size "${RUNTIME_CACHE_SIZE}"
  --max-runtime-instances "${MAX_RUNTIME_INSTANCES}"
)

if [[ -n "${BOOT_NODES:-}" ]]; then
  ARGS+=(--bootnodes ${BOOT_NODES})
fi

if [[ -n "${RESERVED_NODES:-}" ]]; then
  ARGS+=(--reserved-nodes "${RESERVED_NODES}")
fi

if [[ -n "${RESERVED_ONLY:-}" ]]; then
  ARGS+=(--reserved-only)
fi

if [[ -n "${FLAG_LAFA:-}" ]]; then
  ARGS+=(-laleph-party=debug -laleph-network=debug -lnetwork-clique=debug -laleph-finality=debug -laleph-justification=debug -laleph-data-store=debug -laleph-metrics=debug)
fi

if [[ -n "${FLAG_L_ALEPH_BFT:-}" ]]; then
  ARGS+=(-lAlephBFT=debug)
fi

if [[ -n "${PUBLIC_ADDR:-}" ]]; then
  ARGS+=(--public-addr "${PUBLIC_ADDR}")
fi

if [[ "true" == "$ALLOW_PRIVATE_IPV4" ]]; then
  ARGS+=(--allow-private-ipv4)
fi

if [[ "true" == "$DISCOVER_LOCAL" ]]; then
  ARGS+=(--discover-local)
fi

if [[ "false" == "${PROMETHEUS_ENABLED}" ]]; then
  ARGS+=(--no-prometheus)
fi

if [[ "true" == "${PROMETHEUS_ENABLED}" ]]; then
  ARGS+=(--prometheus-external)
fi

if [[ "false" == "${TELEMETRY_ENABLED}" ]]; then
  ARGS+=(--no-telemetry)
fi

if [[ "true" == "${TELEMETRY_ENABLED}" ]]; then
  ARGS+=(--telemetry-url "${TELEMETRY_URL} ${TELEMETRY_VERBOSITY_LVL}")
fi

if [[ "true" == "${VALIDATOR}" ]]; then
  ARGS+=(--rpc-methods Unsafe)
  PUBLIC_VALIDATOR_ADDRESS=${PUBLIC_VALIDATOR_ADDRESS:?'Public validator address should be specified'}
fi

if [[ "false" == "${VALIDATOR}" ]]; then
  ARGS+=(--rpc-methods Safe)
  # We will never use this address, but because of the current shape of our code we need to have something here.
  # This address is one reserved for documentation, so attempting to connect to it should always fail.
  PUBLIC_VALIDATOR_ADDRESS=${PUBLIC_VALIDATOR_ADDRESS:-"192.0.2.1:${VALIDATOR_PORT}"}
fi

ARGS+=(--public-validator-addresses "${PUBLIC_VALIDATOR_ADDRESS}")

if [[ -n "${UNIT_CREATION_DELAY:-}" ]]; then
  ARGS+=(--unit-creation-delay="${UNIT_CREATION_DELAY}")
fi

echo "${CUSTOM_ARGS}" | xargs aleph-node "${ARGS[@]}"
