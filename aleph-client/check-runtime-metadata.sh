#!/bin/bash

set -e

echo "This script must be run from the aleph-client directory."
echo "Make sure you have aleph-node running in the background."
echo "Make sure you have subxt binary installed with"
echo  "cargo install subxt-cli --version 0.30.1 --locked"

SUBXT_BINARY=${SUBXT_BINARY:-"subxt"}

pidof aleph-node || { echo "Error: no aleph-node process found!"; exit 1; }

"${SUBXT_BINARY}" codegen --derive Clone --derive Debug --derive PartialEq --derive Eq \
  --substitute-type 'sp_core::crypto::AccountId32=::subxt::utils::Static<::subxt::ext::sp_core::crypto::AccountId32>' \
  | rustfmt --edition=2021 --config-path rustfmt.toml > aleph_zero.rs;

diff -y -W 200 --suppress-common-lines aleph_zero.rs src/aleph_zero.rs
diff_exit_code=$?
if [[ ! $diff_exit_code -eq 0 ]]; then
  echo "Current runtime metadata is different than versioned in git!"
  echo "Run commands: "
  ecch "  cd aleph-client"
  echo "  ${SUBXT_BINARY} codegen --derive Clone --derive Debug --derive PartialEq --derive Eq \
            --substitute-type 'sp_core::crypto::AccountId32=::subxt::utils::Static<::subxt::ext::sp_core::crypto::AccountId32>' \
            | rustfmt --edition=2021 --config-path rustfmt.toml > src/aleph_zero.rs;"
  echo "  git add src/alephzero.rs && git commit && git push"
  exit 1
fi
echo "Current runtime metadata and versioned in git matches."
