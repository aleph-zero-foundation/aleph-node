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

aleph-e2e-client "${ARGS[@]}"

echo "Done!"
