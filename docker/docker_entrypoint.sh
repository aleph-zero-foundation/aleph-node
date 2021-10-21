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
EXTERNAL_PORT=${EXTERNAL_PORT:-${PORT}}
VALIDATOR=${VALIDATOR:-true}
WS_MAX_CONNECTIONS=${WS_MAX_CONNECTIONS:-100}
POOL_LIMIT=${POOL_LIMIT:-1024}

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
  --rpc-port "${RPC_PORT}" --ws-port "${WS_PORT}" --port "${PORT}"
  --rpc-cors all
  --no-prometheus --no-telemetry # Currently not using. plan to start as soon as capacity is available
  --no-mdns
  --ws-max-connections "${WS_MAX_CONNECTIONS}"
)

if [[ -n "${BOOT_NODES:-}" ]]; then
  ARGS+=(--bootnodes "${BOOT_NODES}")
fi

if [[ -n "${RESERVED_NODES:-}" ]]; then
  ARGS+=(--reserved-nodes "${RESERVED_NODES}")
fi

if [[ -n "${RESERVED_ONLY:-}" ]]; then
  ARGS+=(--reserved-only)
fi

if [[ -n "${FLAG_LAFA:-}" ]]; then
  ARGS+=(-lafa=debug)
fi

if [[ -n "${FLAG_L_ALEPH_BFT:-}" ]]; then
  ARGS+=(-lAlephBFT=debug)
fi

if [[ -n "${PUBLIC_ADDR:-}" ]]; then
  ARGS+=(--public-addr "${PUBLIC_ADDR}")
fi

if [[ "true" == "$ALLOW_PRIVATE_IPV4" ]]; then
  ARGS+=(--allow-private-ipv4 )
fi

if [[ "true" == "$DISCOVER_LOCAL" ]]; then
  ARGS+=(--discover-local)
fi

if [[ "true" == "${VALIDATOR}" ]]; then
    ARGS+=(--unsafe-ws-external --unsafe-rpc-external --rpc-methods Unsafe)
fi

if [[ "false" == "${VALIDATOR}" ]]; then
    ARGS+=(--ws-external --rpc-external --rpc-methods Safe)
fi

aleph-node "${ARGS[@]}"
