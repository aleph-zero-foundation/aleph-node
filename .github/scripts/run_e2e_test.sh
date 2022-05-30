#!/bin/bash

set -eu

while getopts t: opt
do
  case $opt in
    t)
      export TEST_CASE=$OPTARG;;
    \?)
      echo "Invalid option: -$OPTARG"
      exit 1
      ;;
  esac
done

# source docker/env

docker run -v $(pwd)/docker/data:/data --network container:Node0 -e TEST_CASE -e NODE_URL=127.0.0.1:9943 -e RUST_LOG=info aleph-e2e-client:latest

exit $?
