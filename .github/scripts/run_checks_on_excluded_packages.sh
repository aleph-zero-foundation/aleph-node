#!/bin/bash

set -eou pipefail

function usage(){
  cat << EOF
Usage:
   $0
   This script is intended to run in CI in a job which aim is to perform static checks
   on crates that are excluded from aleph-node workspace.
   It runs static code analysis ie cargo clippy. On smart contracts crates, runs
   cargo contract check.

    --packages PACKAGES
        List of aleph-node crates to check, in a below form
        [aleph-client,baby-liminal-extension,benches/payout-stakers, ...]
    [--skip-liminal]
      optional: excluded liminal crates from checks
EOF
  exit 0
}

SKIP_LIMINAL="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --packages)
      PACKAGES="$2"
      shift;shift
      ;;
    --skip-liminal)
      SKIP_LIMINAL="true"
      shift
      ;;
    *)
      echo "Error: Unrecognized argument $1!"
      exit 1
      ;;
  esac
done

if [[ -z "${PACKAGES}" ]]; then
  echo "Error: --packages is required!"
  exit 1
fi

packages_escaped=$(echo "${PACKAGES}" | sed -e 's/,/ /g' | tr -d '[]')
packages=($packages_escaped)

if [[ "${SKIP_LIMINAL}" == "true" ]]; then
  # cargo clippy-liminal-extension is done in _liminal-checks-on-pr.yml
  packages=("${packages[@]/baby-liminal-extension}")
  packages=("${packages[@]/pallets\/baby-liminal}")
  packages=("${packages[@]/poseidon}")
  packages=("${packages[@]/relations\/ark\/src\/proc_macro}")
  packages=("${packages[@]/relations\/ark}")
  packages=("${packages[@]/relations\/jf}")
fi
echo ${packages[@]}

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
