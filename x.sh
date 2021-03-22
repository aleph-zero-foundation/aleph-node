#!/bin/bash

killall -9 aleph-node

set -e

clear

cargo  build -p aleph-node

authorities=(alice bob charlie dave)
authorities=("${authorities[@]::$1}")

for i in ${!authorities[@]}; do
  auth=${authorities[$i]}
  ./target/debug/aleph-node purge-chain --base-path /tmp/$auth --chain local -y
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
    2> $auth.log   & \
done
