#!/bin/env python
import os
import sys
from os.path import abspath, join
from time import sleep, ctime

from chainrunner import Chain, Seq, generate_keys, check_finalized

def printt(s): print(ctime() + ' | ' + s)

# Path to working directory, where chainspec, logs and nodes' dbs are written:
workdir = abspath(os.getenv('WORKDIR', '/tmp/workdir'))
# Path to the aleph-node binary (important use short-session feature):
binary = abspath(os.getenv('ALEPH_NODE_BINARY', join(workdir, 'aleph-node')))

phrases = [f'//{i}' for i in range(6)]
keys = generate_keys(binary, phrases)
all_accounts = list(keys.values())
chain = Chain(workdir)
printt('Bootstraping the chain with binary')
chain.bootstrap(binary,
                all_accounts[:4],
                nonvalidators=all_accounts[4:],
                sudo_account_id=keys[phrases[0]],
                chain_type='local')

chain.set_flags('no-mdns',
                port=Seq(30334),
                ws_port=Seq(9944),
                rpc_port=Seq(9933),
                unit_creation_delay=200,
                execution='Native',
                pruning='archive')
addresses = [n.address() for n in chain]
chain.set_flags(bootnodes=addresses[0], public_addr=addresses)

chain.set_flags_validator('validator')

printt('Starting the chain')
chain.start('aleph')

printt('Waiting for finalization')
chain.wait_for_finalization(0)
printt('Waiting for authorities')
chain.wait_for_authorities()

printt('Killing one validator and one nonvalidator')
chain.stop(nodes=[3, 4])
finalized_before_kill = check_finalized(chain)

printt('Waiting around 2 sessions')
sleep(30 * 2)

finalized_before_start = check_finalized(chain)

# Check if the finalization didn't stop after a kill.
if finalized_before_start[0] - finalized_before_kill[0] < 10:
    printt('Finalization stalled')
    sys.exit(1)

printt('Restarting nodes')
chain.start('aleph', nodes=[3, 4])
printt('Waiting for finalization')
chain.wait_for_finalization(max(finalized_before_start))

finalized_after = check_finalized(chain)

nonvalidator_diff = finalized_after[4] - finalized_before_start[4]
validator_diff = finalized_after[3] - finalized_before_start[3]

delta = 5

# Check if the murdered nodes started catching up with reasonable nr of blocks.
if nonvalidator_diff <= delta:
    printt(f'Too small catch up for nonvalidators: {nonvalidator_diff}')
    sys.exit(1)

if validator_diff <= delta:
    printt(f'Too small catch up for validators: {validator_diff}')
    sys.exit(1)
