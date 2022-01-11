#!/bin/bash

set -e

NETRC_CREDS="./_netrc"
RUNTIME_TOOL="./send_runtime"
SUDO_PHRASE=${RUNTIME_PHRASE}

RPC_ADDR="rpc.dev.azero.dev"
WS_ADDR="ws.dev.azero.dev"

echo -n "Checking runtime version on devnet: "
OLD_VER=$(curl -sS -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}' $RPC_ADDR | jq .result.specVersion)
echo "$OLD_VER"

git clone -q https://github.com/Cardinal-Cryptography/aleph-node.git aleph-node
echo -n "Checking runtime version in latest source: "
NEW_VER=$(grep "spec_version:" aleph-node/bin/runtime/src/lib.rs | grep -o '[0-9]*')
echo "$NEW_VER"

if (( "$NEW_VER" == "$OLD_VER" )); then
    echo "No update needed"
    exit 0
fi

if (( "$NEW_VER" >= "$OLD_VER" )); then
    echo -n "Fetching latest runtime from github..."
    ALEPH_RUNTIME_URL=$(curl -sS -H "Accept: application/vnd.github.v3+json" https://api.github.com/repos/Cardinal-Cryptography/aleph-node/actions/artifacts | jq '.artifacts' | jq -r '.[] | select(.name=="aleph-runtime") | .archive_download_url' | head -n 1)
    curl -sS --netrc-file $NETRC_CREDS -L -o aleph-runtime.zip $ALEPH_RUNTIME_URL
    echo "completed"
    mkdir runtime
    unzip aleph-runtime.zip -d runtime
    NEW_RUNTIME=runtime/$(ls runtime)

    echo -n "Sending runtime update... "
    $RUNTIME_TOOL --url $WS_ADDR --sudo-phrase $SUDO_PHRASE $NEW_RUNTIME
    echo "completed"
    echo -n "Checking new runtime version on devnet: "
    UPD_VER=$(curl -sS -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}' $RPC_ADDR | jq .result.specVersion)
    echo "$UPD_VER"
    if (( $NEW_VER != $UPD_VER )); then
        echo "ERROR: runtime update failed"
        exit 1
    fi
    echo "SUCCESS: runtime updated"
fi
