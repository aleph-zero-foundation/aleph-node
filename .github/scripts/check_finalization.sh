#!/bin/bash

RPC_HOST=127.0.0.1
RPC_PORT=9933
LAST_FINALIZED=""
VALIDATOR=damian

while [[ "$LAST_FINALIZED" =~ "0x0" ]] || [[ -z "$LAST_FINALIZED" ]]; do
  block_hash=$(docker run --network container:$VALIDATOR appropriate/curl:latest \
                      -H "Content-Type: application/json" \
                      -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getFinalizedHead"}' http://$RPC_HOST:$RPC_PORT | jq '.result')
  ret_val=$?
  if [ $ret_val -ne 0 ]; then
    echo "failed calling the `chain_getFinalizedHead` method" >&2
    continue
  fi

  finalized_block=$(docker run --network container:$VALIDATOR appropriate/curl:latest \
                           -H "Content-Type: application/json" \
                           -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock", "params": ['$block_hash']}' http://$RPC_HOST:$RPC_PORT | jq '.result.block.header.number')

  ret_val=$?
  if [ $ret_val -ne 0 ]; then
    echo "failed calling the `chain_getBlock` method" >&2
    continue
  else
    LAST_FINALIZED=$finalized_block
    echo "Last finalized block number: $LAST_FINALIZED"
  fi

done

exit $?
