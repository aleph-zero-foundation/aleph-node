#!/bin/env python
import os
import sys
from os.path import abspath, join
from time import sleep, ctime
import argparse

from chainrunner import Chain, Seq, generate_keys, check_finalized


def printt(s): print(ctime() + ' | ' + s)


argParser = argparse.ArgumentParser()
argParser.add_argument("--state-pruning", help="state pruning argument")
state_pruning = argParser.parse_args().state_pruning

# Path to working directory, where chainspec, logs and nodes' dbs are written:
workdir = abspath(os.getenv('WORKDIR', '/tmp/workdir'))
# Path to the aleph-node binary (important DON'T use short-session feature):
binary = abspath(os.getenv('ALEPH_NODE_BINARY', join(workdir, 'aleph-node')))

phrases = [f'//{i}' for i in range(4)]
keys = generate_keys(binary, phrases)
chain = Chain(workdir)
printt('Bootstrapping the chain with binary')
chain.bootstrap(binary,
                keys.values(),
                sudo_account_id=keys[phrases[0]],
                chain_type='local')

chain.set_flags('no-mdns',
                port=Seq(30334),
                validator_port=Seq(30343),
                rpc_port=Seq(9944),
                unit_creation_delay=200,
                execution='Native')
addresses = [n.address() for n in chain]
validator_addresses = [n.validator_address() for n in chain]
chain.set_flags(bootnodes=addresses[0])
chain.set_flags_validator(public_addr=addresses,
                          public_validator_addresses=validator_addresses)
if state_pruning is not None:
    chain.set_flags('experimental-pruning', state_pruning=state_pruning,
                    )

chain.set_flags_validator('validator')

printt('Starting the chain')
chain.start('aleph')

printt('Waiting for finalization')
chain.wait_for_finalization(0)
printt('Waiting for authorities')
chain.wait_for_authorities(timeout=120)
if state_pruning is not None and state_pruning.isnumeric():
    bound = min(256, int(state_pruning))
    printt(f'Pruning turned on. Waiting for {bound} blocks to finalize')
    chain.wait_for_finalization(bound)

delta = 5

for sleep_duration in [21, 37, 15]:
    printt('Killing one validator')
    chain[3].stop()
    finalized_before_kill = check_finalized(chain)

    printt(f'Waiting {sleep_duration}s')
    sleep(sleep_duration)

    finalized_before_start = check_finalized(chain)

    # Check if the finalization didn't stop after a kill.
    if finalized_before_start[0] - finalized_before_kill[0] < delta:
        printt('Finalization stalled')
        sys.exit(1)

    printt('Restarting nodes')
    chain.start('aleph', nodes=[3])

    printt('Waiting for finalization')
    chain.wait_for_finalization(finalized_before_start[3], nodes=[3])

    finalized_after = check_finalized(chain)
    diff = finalized_after[3] - finalized_before_start[3]

    # Check if the murdered node started catching up with reasonable nr of blocks.
    if diff <= delta:
        printt(f'Too small catch up for validators: {diff}')
        sys.exit(1)
