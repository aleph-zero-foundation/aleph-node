#!/bin/bash

set -eo pipefail

if [[ -n "${1}" ]]; then
  # if the first arg passed (run via CI), it is of below form
  # [aleph-client,baby-liminal-extension,benches/payout-stakers, ...]
  list=$(echo "${1}" | sed -e 's/,/ /g' | tr -d '[]')
  packages=($list)
else
  TOML_FILE="Cargo.toml"

  # Read the TOML file and extract the `exclude` entries
  packages=$(awk -F ' *= *' '/^exclude *= *\[/ {found=1} found && /^\]$/ {found=0} found' "$TOML_FILE")
  packages="$(echo ${packages} | sed 's/[][,]/ /g' | sed 's/\s\+/\n/g' | sed '/^$/d')"

  # Remove leading and trailing whitespace, and quotes from the entries
  packages=$(echo "$packages" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e 's/^"//' -e 's/"$//')

  packages="${packages//'%0A'/$'\n'}"

  # Remove the key
  packages=${packages:10}
fi

# cargo clippy-liminal-extension is done in _liminal-checks-on-pr.yml
packages=("${packages[@]/baby-liminal-extension}")

for p in ${packages[@]}; do
  echo "Checking package $p ..."
  pushd "$p"

  if [[ "$p" =~ .*contracts.* ]]; then
     docker run \
      --network host \
      -v "$PWD:/code" \
      -u "$(id -u):$(id -g)" \
      --name ink_builder \
      --platform linux/amd64 \
      --rm public.ecr.aws/p6e8q1z1/ink-dev:2.0.0 cargo contract check
  else
    cargo clippy -- --no-deps -D warnings
  fi
  popd
done
