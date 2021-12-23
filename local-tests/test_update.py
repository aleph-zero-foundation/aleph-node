#!/bin/env python

import subprocess
from os.path import join
from time import sleep

from chainrunner import Chain, Seq, generate_keys

# Path to working directory, where chainspec, logs and nodes' dbs are written:
WORKDIR = '/tmp/workdir'
# Path to the pre-update aleph-node binary:
oldbin = join(WORKDIR, 'aleph-node-old')
# Path to the post-update aleph-node binary:
newbin = join(WORKDIR, 'aleph-node-new')
# Path to the post-update compiled runtime:
runtime = join(WORKDIR, 'aleph_runtime.compact.wasm')
# Path to the send-runtime binary (which lives in aleph-node/local-tests/send-runtime):
SEND_RUNTIME = 'send-runtime/target/release/send_runtime'


def query_runtime_version(nodes):
    print('Current version:')
    for i, node in enumerate(nodes):
        sys = node.rpc('system_version').result
        rt = node.rpc('state_getRuntimeVersion').result['specVersion']
        print(f'  Node {i}: system: {sys}  runtime: {rt}')


def check_highest(nodes):
    results = [node.highest_block() for node in nodes]
    highest, finalized = zip(*results)
    print('Blocks seen by nodes:')
    print('  Highest:   ', *highest)
    print('  Finalized: ', *finalized)


phrases = ['//Cartman', '//Stan', '//Kyle', '//Kenny']
keys = generate_keys(newbin, phrases)

chain = Chain(WORKDIR)
print('Bootstraping the chain with old binary')
chain.bootstrap(oldbin,
                keys.values(),
                sudo_account_id=keys[phrases[0]],
                chain_type='local',
                millisecs_per_block=2000,
                session_period=40)

chain.set_flags('validator',
                port=Seq(30334),
                ws_port=Seq(9944),
                rpc_port=Seq(9933),
                unit_creation_delay=200,
                execution='Native')

print('Starting the chain with old binary')
chain.start('old')

print('Waiting a minute')
sleep(60)

check_highest(chain)
query_runtime_version(chain)

print('Killing node 3 and deleting its database')
chain[3].stop()  # OH MY GOD THEY KILLED KENNY!
chain[3].purge()

print('Restarting node 3 with new binary')
chain[3].binary = newbin
chain[3].start('new3')

print('Waiting a minute')
sleep(60)

check_highest(chain)
query_runtime_version(chain)

print('Submitting extrinsic with new runtime')
subprocess.check_call(
    [SEND_RUNTIME, '--url', 'localhost:9945', '--sudo-phrase', phrases[0], runtime])

print('Waiting a bit')
sleep(15)

check_highest(chain)
query_runtime_version(chain)

print('Restarting remaining nodes with new binary')
chain.stop(nodes=[0, 1, 2])
chain.set_binary(newbin, nodes=[0, 1, 2])
chain.start('new', nodes=[0, 1, 2])

print('Waiting a minute')
sleep(60)

check_highest(chain)
query_runtime_version(chain)

print('Stopping the chain')
chain.stop()

hf = min(node.highest_block()[1] for node in chain)
print(f'Sanity check: the highest finalized block is {hf}. '
      f'Comparing exported states after that block:')
if chain[0].state(hf) == chain[1].state(hf) == chain[2].state(hf) == chain[3].state(hf):
    print("The same :)")
else:
    print("DIFFERENT!")
