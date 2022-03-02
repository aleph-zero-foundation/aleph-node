#!/usr/bin/env bash
set -euo pipefail

SPAWN_SHELL=${SPAWN_SHELL:-false}
SHELL_NIX_FILE=${SHELL_NIX_FILE:-"default.nix"}
DYNAMIC_LINKER_PATH=${DYNAMIC_LINKER_PATH:-"/lib64/ld-linux-x86-64.so.2"}

while getopts "s" flag
do
    case "${flag}" in
        s) SPAWN_SHELL=true;;
        *)
            usage
            exit
            ;;
    esac
done

function usage(){
    echo "Usage:
      ./nix-build.sh [-s - spawn nix-shell]"
}

if [ $SPAWN_SHELL = true ]
then
    nix-shell --pure $SHELL_NIX_FILE
else
    nix-build $SHELL_NIX_FILE
    # we need to change the dynamic linker
    # otherwise our binary references one that is specific for nix
    patchelf --set-interpreter $DYNAMIC_LINKER_PATH ./result/bin/aleph-node
    mv ./result/bin/aleph-node ./
fi
