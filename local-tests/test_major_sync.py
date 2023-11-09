#!/bin/env python

import os
from os.path import abspath, join
import logging
from chainrunner import Chain, Seq, generate_keys, check_finalized

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s %(levelname)-8s %(message)s',
)

workdir = abspath(os.getenv('WORKDIR', '/tmp/workdir'))
logging.info(f"Workdir is {workdir}")
binary = abspath(os.getenv('ALEPH_NODE_BINARY', join(workdir, 'aleph-node')))
logging.info(f"aleph-node binary is {binary}")

phrases = [f'//{i}' for i in range(5)]
keys = generate_keys(binary, phrases)
chain = Chain(workdir)
logging.info('Bootstrapping the chain with binary')

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

chain.set_flags_validator('validator')

logging.info('Starting the chain')
chain.start('aleph', nodes=[0, 1, 2, 3])
logging.info('Waiting for 2700 blocks to finalize (3 sessions) for nodes 0-3')
chain.wait_for_finalization(old_finalized=0,
                            nodes=[0, 1, 2, 3],
                            finalized_delta=2700,
                            catchup=True,
                            catchup_delta=5,
                            timeout=60 * 60)
check_finalized(chain)
logging.info('Starting 4th node')
chain.start('aleph', nodes=[4])
logging.info('Waiting for 4th node to catch up')
chain.wait_for_finalization(old_finalized=0,
                            nodes=[4],
                            finalized_delta=2700,
                            catchup=True,
                            catchup_delta=5,
                            timeout=10 * 60)
check_finalized(chain)
logging.info('OK')
