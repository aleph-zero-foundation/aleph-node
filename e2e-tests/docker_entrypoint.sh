#!/usr/bin/env bash
set -euo pipefail

ARGS=(
  --node "${NODE_URL}"
)

if [[ -n "${TEST_CASES:-}" ]]; then
  ARGS+=(--test-cases "${TEST_CASES}")
fi

# If test case params are both not empty, run client with them. Otherwise, run without params.
if [[ -n "${RESERVED_SEATS:-}" && -n "${NON_RESERVED_SEATS:-}" ]]; then
  ARGS+=(
    --reserved-seats "${RESERVED_SEATS}"
    --non-reserved-seats "${NON_RESERVED_SEATS}"
  )
fi

if [[ -n "${UPGRADE_VERSION:-}" && -n "${UPGRADE_SESSION:-}" && -n "${UPGRADE_FINALIZATION_WAIT_SESSIONS:-}" ]]; then
    ARGS+=(
        --upgrade-to-version "${UPGRADE_VERSION}"
        --upgrade-session "${UPGRADE_SESSION}"
        --upgrade-finalization-wait-sessions "${UPGRADE_FINALIZATION_WAIT_SESSIONS}"
    )
fi

E2E_CONFIG="${ARGS[*]}" aleph-e2e-client $TEST_CASES --nocapture

echo "Done!"
