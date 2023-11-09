#!/usr/bin/env bash

# This script is used for running e2e tests via python framework, and intended to run in CI

set -euo pipefail

function usage(){
  cat << EOF
Usage:
   $0
    --aleph-node BINARY]
      path to aleph-node-binary
    --testcase NAME
      name of python file in local-tests directory to run
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --aleph-node)
      ALEPH_NODE_BINARY="$2"
      shift;shift
      ;;
    --testcase)
      TESTCASE="$2"
      shift;shift
      ;;
    --help)
      usage
      shift
      ;;
    *)
      echo "Unrecognized argument $1!"
      ;;
  esac
done

# relative paths should be improved in python scripts CI, as it matters that below line is
pushd local-tests/ > /dev/null

if [[ ! -f "${ALEPH_NODE_BINARY}" ]]; then
  echo "Error: aleph-node binary does not exist at given path ${ALEPH_NODE_BINARY}."
  exit 1
fi
if [[ -z "${TESTCASE}" ]]; then
  echo "Error: TESTCASE name must not be empty."
  exit 1
fi

file_name_to_run="${TESTCASE}.py"
if [[ ! -x "${file_name_to_run}" ]]; then
  echo "Error: testcase ${file_name_to_run} does not exist or it's not executable."
  popd > /dev/null
  exit 1
fi

chmod +x "${ALEPH_NODE_BINARY}"
echo "Installing python requirements"
pip install -r requirements.txt

# https://stackoverflow.com/questions/59812009/what-is-the-use-of-pythonunbuffered-in-docker-file
# Setting PYTHONUNBUFFERED to a non-empty value different from 0 ensures that the python output i.e.
# the stdout and stderr streams are sent straight to terminal (e.g. your container log) without being
# first buffered and that you can see the output of your application.
export PYTHONUNBUFFERED=y
export ALEPH_NODE_BINARY
export WORKDIR=$(mktemp -d)
eval "./${file_name_to_run}"
popd > /dev/null
