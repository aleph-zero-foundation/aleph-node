#!/usr/bin/env bash

# an ugly workaround for the fact we want to ignore rustdoc warnings in generated runtime file
echo "#[doc(hidden)]" > aleph_zero.rs
subxt codegen --derive Clone --derive Debug --derive Eq --derive PartialEq | rustfmt --edition=2021 --config-path aleph-node/rustfmt.toml >> aleph_zero.rs

diff -y -W 200 --suppress-common-lines aleph_zero.rs aleph-node/aleph-client/src/aleph_zero.rs
diff_exit_code=$?
if [[ ! $diff_exit_code -eq 0 ]]; then
  echo "Current runtime metadata is different than versioned in git!"
  echo "Run subxt codegen --derive Clone --derive Debug --derive Eq --derive PartialEq | rustfmt --edition=2021 >" \
"src/aleph_zero.rs from aleph-client directory and commit to git."
   exit 1
fi
echo "Current runtime metadata and versioned in git matches."
