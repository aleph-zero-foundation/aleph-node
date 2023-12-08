#!/usr/bin/env bash

set -euo pipefail

function usage {
    cat << EOF
Usage:
  $0
    -t
      Test cases to run.
    -f
      Number of reserved seats available to validators, ignored if empty or `-n` is empty.
    -n
      Number of non-reserved seats available to validators, ignored if empty or `-f` is empty.
EOF
  exit 0
}

while getopts "h:t:f:n:a:" flag
do
  case "${flag}" in
    h) usage;;
    t) TEST_CASES="${OPTARG}";;
    f) RESERVED_SEATS="${OPTARG}";;
    n) NON_RESERVED_SEATS="${OPTARG}";;
    a) ALEPH_E2E_CLIENT_IMAGE="${OPTARG}";;
    *)
      echo "Unrecognized argument "${flag}"!"
      usage
      exit 1
      ;;
  esac
done

# This is arbitrary.
MAX_VALIDATOR_COUNT=20

NODE_URL=${NODE_URL:-"ws://127.0.0.1:9944"}
NETWORK=${NETWORK:-"container:Node0"}
ALEPH_E2E_CLIENT_IMAGE=${ALEPH_E2E_CLIENT_IMAGE:-"aleph-e2e-client:dev"}

ARGS=(
  --network "${NETWORK}"
  -e NODE_URL="${NODE_URL}"
  -e RUST_LOG=info
  -e VALIDATOR_COUNT
)

if [[ -n "${TEST_CASES:-}" ]]; then
  ARGS+=(-e TEST_CASES="${TEST_CASES}")
fi

RESERVED_SEATS="${RESERVED_SEATS:-}"
NON_RESERVED_SEATS="${NON_RESERVED_SEATS:-}"

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

if [[ -n "${OUT_LATENCY:-}" ]]; then
    ARGS+=(-e OUT_LATENCY)
fi
timeout_duration="${TIMEOUT_MINUTES:-15m}"
echo "Running test, logs will be shown when tests finishes or after ${timeout_duration} timeout."
# a hack to set global timeout on a e2e testcase run
# we can't do that on GH yaml level due to https://github.com/actions/runner/issues/1979
docker_service=$(docker run -v "$(pwd)/contracts:/contracts" -v "$(pwd)/docker/data:/data" -d "${ARGS[@]}" \
    "${ALEPH_E2E_CLIENT_IMAGE}")
set +e
timeout_output=$(timeout "${timeout_duration}" docker wait "${docker_service}")
docker_exit_code=$?
# timeout returns 124 exit code if command times out
# otherwise, docker wait finishes and it prints docker service exit code on stdout
if [[ "${docker_exit_code}" != 124 ]]; then
  docker_exit_code="${timeout_output}"
fi
echo "Test exited with exit code ${docker_exit_code}"
echo "Logs from test:"
docker logs "${docker_service}"
exit "${docker_exit_code}"
