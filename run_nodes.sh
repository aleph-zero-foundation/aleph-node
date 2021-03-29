#!/bin/bash

killall -9 aleph-node

set -e

clear

cargo  build -p aleph-node

authorities=(alice bob charlie dave)
authorities=("${authorities[@]::$1}")

for i in ${!authorities[@]}; do
  auth=${authorities[$i]}
  rm -rf /tmp/$auth
done

for i in ${!authorities[@]}; do
  auth=${authorities[$i]}
  ./target/debug/aleph-node \
    --validator \
    --chain local \
    --base-path /tmp/$auth \
    --$auth \
    --ws-port $(expr 9944 + $i) \
    --port $(expr 30334 + $i) \
    --execution Native \
    -lrush=debug \
    -lafa=debug \
    2> $auth.log   & \
done
