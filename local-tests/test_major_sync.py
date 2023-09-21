#!/bin/env python
import os
import sys
from os.path import abspath, join
from time import sleep, ctime

from chainrunner import Chain, Seq, generate_keys, check_finalized


def printt(s): print(ctime() + ' | ' + s)


workdir = abspath(os.getenv('WORKDIR', '/tmp/workdir'))
binary = abspath(os.getenv('ALEPH_NODE_BINARY', join(workdir, 'aleph-node')))

phrases = [f'//{i}' for i in range(5)]
keys = generate_keys(binary, phrases)
all_accounts = list(keys.values())
chain = Chain(workdir)

chain.new(binary, all_accounts)

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

chain.set_flags_validator('validator')

printt('Starting the chain')
chain.start('aleph', nodes=[0, 1, 2, 3])

# sleep 1 min so the nodes 0-3 have some time to start up
sleep(60)
chain.start('aleph', nodes=[4])

printt('Waiting around 10 mins')
sleep(10 * 60)

finalized = check_finalized(chain)

catching_up_validator_finalized = finalized[4]
normal_validator_finalized = finalized[3]

if normal_validator_finalized < 3 * 1800:
    printt(f'Not enough finalized blocks in the test time {normal_validator_finalized}')
    sys.exit(1)

# Check if the late node catched up to other validators
if abs(catching_up_validator_finalized - normal_validator_finalized) > 5:
    printt(f'Too small catch up for late node: got: {catching_up_validator_finalized} expected: {normal_validator_finalized}')
    sys.exit(1)
