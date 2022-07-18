#!/bin/env python
import os
import sys
from os.path import abspath, join
from time import sleep, ctime

from chainrunner import Chain, Seq, generate_keys, check_finalized, check_version

def printt(s): print(ctime() + ' | ' + s)

# Path to working directory, where chainspec, logs and nodes' dbs are written:
workdir = abspath(os.getenv('WORKDIR', '/tmp/workdir'))
# Path to the pre-update aleph-node binary:
oldbin = abspath(os.getenv('OLD_BINARY', join(workdir, 'aleph-node-old')))
# Path to the post-update aleph-node binary:
newbin = abspath(os.getenv('NEW_BINARY', join(workdir, 'aleph-node-new')))
# Path to the post-update compiled runtime:
runtime = abspath(os.getenv('NEW_RUNTIME', join(workdir, 'aleph_runtime.compact.wasm')))
# Path to cliain:
cliain = abspath('../bin/cliain/target/release/cliain')

phrases = ['//Cartman', '//Stan', '//Kyle', '//Kenny']
keys = generate_keys(newbin, phrases)
chain = Chain(workdir)
printt('Bootstraping the chain with old binary')
chain.bootstrap(oldbin,
                keys.values(),
                sudo_account_id=keys[phrases[0]],
                chain_type='local')

chain.set_flags('validator',
                port=Seq(30334),
                ws_port=Seq(9944),
                rpc_port=Seq(9933),
                unit_creation_delay=200,
                execution='Native',
                pruning='archive')

addresses = [n.address() for n in chain]
chain.set_flags(public_addr=addresses)

printt('Starting the chain with old binary')
chain.start('old', backup=False)

printt('Waiting for finalization')
chain.wait_for_finalization(0)

check_version(chain)
last_finalized = max(check_finalized(chain))

printt('Killing node 3 and deleting its database')
chain[3].stop()  # OH MY GOD THEY KILLED KENNY!
chain[3].purge()

printt('Restarting node 3 with new binary')
chain[3].binary = newbin
chain[3].start('new3')
printt('Waiting for finalization')
chain.wait_for_finalization(last_finalized, nodes=[3])

oldver = check_version(chain)
check_finalized(chain)

printt('Submitting extrinsic with new runtime')
chain.update_runtime(cliain, phrases[0], runtime)

printt('Waiting 10s')
sleep(10)

newver = check_version(chain)
last_finalized = max(check_finalized(chain))

printt('Killing remaining nodes')
chain.stop(nodes=[0, 1, 2])
chain.set_binary(newbin, nodes=[0, 1, 2])
printt('Waiting 10s')
sleep(10)

printt('Restarting remaining nodes with new binary')
chain.start('new', nodes=[0, 1, 2])
printt('Waiting for finalization')
chain.wait_for_finalization(last_finalized)

check_finalized(chain)
check_version(chain)

printt('Stopping the chain')
chain.stop()

printt('Waiting 10s')
sleep(10)

hf = min(node.highest_block()[1] for node in chain)
printt(f'Sanity check: the highest finalized block is {hf}. '
      f'Comparing exported states after that block:')
if chain[0].state(hf) == chain[1].state(hf) == chain[2].state(hf) == chain[3].state(hf):
    printt("The same :)")
else:
    printt("DIFFERENT!")
    sys.exit(1)

if oldver == newver:
    printt("ERROR: runtime version reported by nodes didn't change after the update")
    sys.exit(1)
