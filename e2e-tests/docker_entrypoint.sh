#!/usr/bin/env bash
set -euo pipefail

aleph-e2e-client --node "$NODE_URL" --test-cases "$TEST_CASE"

echo "Done!"
