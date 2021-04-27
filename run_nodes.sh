#!/bin/bash

if [ -z "$1" ]
then
    echo "The committee size is missing, usage:

    ./run_nodes.sh SIZE

where 2 <= SIZE <= 8"
    exit
fi

killall -9 aleph-node

set -e

clear

echo "$1" > /tmp/n_members

cargo build -p aleph-node

authorities=(Damian Tomasz Zbyszko Hansu Adam Matt Antoni Michal)
authorities=("${authorities[@]::$1}")

./target/debug/aleph-node dev-keys  --base-path /tmp --chain dev --key-types aura alp0

for i in ${!authorities[@]}; do
  auth=${authorities[$i]}
  ./target/debug/aleph-node purge-chain --base-path /tmp/$auth --chain dev -y
  ./target/debug/aleph-node \
    --validator \
    --chain dev \
    --base-path /tmp/$auth \
    --name $auth \
    --ws-port $(expr 9944 + $i) \
    --port $(expr 30334 + $i) \
    --execution Native \
    2> $auth-$i.log  & \
done
