#!/bin/bash

set -e

NETRC_CREDS="./_netrc"
CLIAIN="./cliain"
SUDO_PHRASE=${RUNTIME_PHRASE}

RPC_ADDR="${RPC_ENVIRONMENT_ENDPOINT:-rpc.dev.azero.dev}"
WS_ADDR="${WS_ENVIRONMENT_ENDPOINT:-ws.dev.azero.dev}"

echo "Heating up for 10s"
sleep 10

echo -n  $(date +"%d-%b-%y %T") "   Checking runtime version on devnet: "
OLD_VER=$(curl -sS -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}' $RPC_ADDR | jq .result.specVersion)
echo "$OLD_VER"

git clone -q https://github.com/Cardinal-Cryptography/aleph-node.git aleph-node
echo -n $(date +"%d-%b-%y %T") "   Checking runtime version in latest source: "
NEW_VER=$(grep "spec_version:" aleph-node/bin/runtime/src/lib.rs | grep -o '[0-9]*')
echo "$NEW_VER"

if (( "$NEW_VER" == "$OLD_VER" )); then
    echo $(date +"%d-%b-%y %T") "   No update needed"
    exit 0
fi

if (( "$NEW_VER" > "$OLD_VER" )); then
    echo -n $(date +"%d-%b-%y %T") "   Fetching latest runtime from github..."
    ALEPH_RUNTIME_URL=$(curl -sS -H "Accept: application/vnd.github.v3+json" https://api.github.com/repos/Cardinal-Cryptography/aleph-node/actions/artifacts | jq -r '.artifacts[] | select(.name=="aleph-release-runtime").archive_download_url' | head -n 1)
    curl -sS --netrc-file $NETRC_CREDS -L -o aleph-runtime.zip $ALEPH_RUNTIME_URL
    echo "completed"
    mkdir runtime
    unzip aleph-runtime.zip -d runtime
    NEW_RUNTIME=runtime/$(ls runtime)

    echo -n $(date +"%d-%b-%y %T") "   Sending runtime update... "
    export RUST_LOG="warn"
    $CLIAIN --node $WS_ADDR --seed "$SUDO_PHRASE" update-runtime --runtime $NEW_RUNTIME
    echo "completed"
    echo -n $(date +"%d-%b-%y %T") "   Checking new runtime version on devnet: "
    sleep 10
    UPD_VER=$(curl -sS -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}' $RPC_ADDR | jq .result.specVersion)
    echo "$UPD_VER"
    if (( $NEW_VER != $UPD_VER )); then
        echo $(date +"%d-%b-%y %T") "   ERROR: runtime update failed"
        exit 1
    fi
    echo $(date +"%d-%b-%y %T") "   SUCCESS: runtime updated"
fi
