#!/usr/bin/env bash
set -euo pipefail

aleph-e2e-client $TEST_CASES --nocapture --test-threads 1

echo "Done!"
