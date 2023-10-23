#!/bin/env python
import os
from os.path import abspath, join
from time import ctime
from chainrunner import Chain, Seq, generate_keys


def printt(s): print(ctime() + ' | ' + s)


# Path to working directory, where chainspec, logs and nodes' dbs are written:
workdir = abspath(os.getenv('WORKDIR', '/tmp/workdir'))
# Path to the aleph-node binary (important DON'T use short-session feature):
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

BLOCKS_PER_STAGE = 50
chain.set_flags_validator('validator')
chain.set_flags('max_nonfinalized_blocks', max_nonfinalized_blocks=BLOCKS_PER_STAGE)

printt('Starting the chain')
chain.start('aleph')
part1 = [0, 2, 4, 6]  # Node ids partitioned into two halves
part2 = [1, 3, 5, 7]

chain.wait_for_finalization(BLOCKS_PER_STAGE, catchup=True, catchup_delta=5)  # run normally for some time

printt('Stopping nodes: ' + ' '.join([str(n) for n in part2]))
chain.stop(nodes=part2)
f1 = chain.get_highest_finalized(nodes=part1)
chain.wait_for_imported_at_height(f1 + BLOCKS_PER_STAGE, nodes=part1)

printt('Stopping nodes: ' + ' '.join([str(n) for n in part1]))
chain.stop(nodes=part1)

f2 = chain.get_highest_finalized(nodes=part2)  # highest finalized before stop
printt('Starting nodes: ' + ' '.join([str(n) for n in part2]))
chain.start('aleph-recovered', nodes=part2)
chain.wait_for_imported_at_height(f2 + BLOCKS_PER_STAGE, nodes=part2)

printt('Starting nodes: ' + ' '.join([str(n) for n in part1]))
chain.start('aleph-recovered', nodes=part1)
chain.wait_for_finalization(0, catchup=True, catchup_delta=5)  # wait for finalization catchup

print('Ok')
