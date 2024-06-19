#!/usr/bin/env bash

echo "This script must be run from the aleph-client directory."
SUBXT_BINARY=${SUBXT_BINARY:-"subxt"}

"${SUBXT_BINARY}" codegen --derive Clone --derive Debug --derive PartialEq --derive Eq \
  --substitute-type 'sp_core::crypto::AccountId32=::subxt::utils::Static<::subxt::ext::sp_core::crypto::AccountId32>' \
  | rustfmt --edition=2021 --config-path rustfmt.toml > aleph_zero.rs;

diff -y -W 200 --suppress-common-lines aleph_zero.rs src/aleph_zero.rs
diff_exit_code=$?
if [[ ! $diff_exit_code -eq 0 ]]; then
  echo "Current runtime metadata is different than versioned in git!"
  echo "Run subxt codegen command as in $(basename $0) from aleph-client directory and commit to git."
  exit 1
fi
echo "Current runtime metadata and versioned in git matches."
