#!/bin/bash

if [ -z "$1" ] || (("$1" < 2 || "$1" > 8))
then
    echo "The committee size is missing, usage:

    ./run_nodes.sh SIZE [Additional Arguments to ./target/debug/aleph-node]

where 2 <= SIZE <= 8"
    exit
fi

killall -9 aleph-node

set -e

clear

n_members="$1"
echo "$n_members" > /tmp/n_members
shift

# cargo build --release -p aleph-node

authorities=(Damian Tomasz Zbyszko Hansu Adam Matt Antoni Michal)
authorities=("${authorities[@]::$n_members}")

./target/release/aleph-node dev-keys  --base-path /tmp --chain dev --key-types aura alp0

for i in ${!authorities[@]}; do
  auth=${authorities[$i]}
  ./target/release/aleph-node purge-chain --base-path /tmp/"$auth" --chain dev -y
  ./target/release/aleph-node \
    --validator \
    --chain dev \
    --base-path /tmp/$auth \
    --name $auth \
    --rpc-port $(expr 9933 + $i) \
    --ws-port $(expr 9944 + $i) \
    --port $(expr 30334 + $i) \
    --execution Native \
    -lafa=debug \
    "$@" \
    2> $auth-$i.log  > aleph-node.log & \
done
