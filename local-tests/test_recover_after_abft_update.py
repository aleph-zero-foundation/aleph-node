#!/bin/env python
import os
from os.path import abspath, join
from time import ctime, sleep
from chainrunner import Chain, Seq, generate_keys


def printt(s): print(ctime() + ' | ' + s)


'''
Make sure to compile the binary with --features short_session
'''

# Path to working directory, where chainspec, logs and nodes' dbs are written:
workdir = abspath(os.getenv('WORKDIR', '/tmp/workdir'))
# Path to the aleph-node binary.
binary = abspath(os.getenv('ALEPH_NODE_BINARY', join(workdir, 'aleph-node')))

phrases = [f'//{i}' for i in range(8)]
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
chain.set_flags(bootnodes=addresses[1])
chain.set_flags_validator(public_addr=addresses, public_validator_addresses=validator_addresses)

BLOCKS_PER_STAGE = 180
chain.set_flags_validator('validator')

printt('Starting the chain')
chain.start('aleph')

chain.wait_for_finalization(10, catchup=True, catchup_delta=5)  # run normally for short time

result = chain.nodes[0].update_finality_version(session=3, sudo_phrase='//0')  # update will happen at block 90
assert result.is_success

chain.wait_for_finalization(BLOCKS_PER_STAGE, catchup=True, catchup_delta=5)  # run normally for around 1 session after updating abft

printt('Stopping all nodes')
chain.stop(nodes=range(8))

sleep(10)

printt('Starting all nodes except one')
chain.start('aleph-recovered', nodes=range(7))  # restart all except the last

f1 = chain.get_highest_finalized()
assert f1 >= BLOCKS_PER_STAGE

chain.wait_for_finalization(2 * BLOCKS_PER_STAGE, catchup=True, catchup_delta=5, nodes=range(7))
f2 = chain.get_highest_finalized(nodes=range(7))
assert f2 >= 2 * BLOCKS_PER_STAGE

print('Ok')
