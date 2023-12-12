#!/bin/bash

set -e

DIR=$(dirname "$0")

"$DIR"/run_checks_on_aleph_node.sh

TOML_FILE="Cargo.toml"
# Read the TOML file and extract the `exclude` entries
packages=$(awk -F ' *= *' '/^exclude *= *\[/ {found=1} found && /^\]$/ {found=0} found' "$TOML_FILE")
packages="$(echo ${packages} | sed 's/[][,]/ /g' | sed 's/\s\+/\n/g' | sed '/^$/d')"
# Remove leading and trailing whitespace, and quotes from the entries
packages=$(echo "$packages" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e 's/^"//' -e 's/"$//')
packages="${packages//'%0A'/$'\n'}"
# Remove the 'exclude' key
packages=${packages:10}
"$DIR"/.github/scripts/run_checks_on_excluded_packages.sh --packages ${packages[@]}

"$DIR"/run_local_e2e_pipeline.sh
