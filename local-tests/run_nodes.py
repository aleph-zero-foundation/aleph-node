#!/bin/env python

# Short script demonstrating the basic usage of `chainrunner` package.
# Reproduces (more or less) the behavior of `run_nodes.sh`.
# For running local experiments it's much more convenient to manage the chain
# using an interactive environment (Python console, Jupyter notebook etc.)

from time import sleep

from chainrunner import Chain, Seq, generate_keys

NODES = 4
WORKDIR = '.'
BINARY = '../target/release/aleph-node'

phrases = ['//Alice', '//Bob', '//Cedric', '//Dick', '//Ezekiel', '//Fanny', '//George', '//Hugo']
keys_dict = generate_keys(BINARY, phrases)
keys = list(keys_dict.values())
NODES = min(NODES, len(phrases))

chain = Chain(WORKDIR)

print(f'Bootstrapping chain for {NODES} nodes')
chain.bootstrap(BINARY,
                keys[:NODES],
                chain_type='local',
                millisecs_per_block=2000,
                session_period=40)
chain.set_flags('validator',
                port=Seq(30334),
                ws_port=Seq(9944),
                rpc_port=Seq(9933),
                unit_creation_delay=200,
                execution='Native')

print('Starting the chain')
chain.start('node')

print('Waiting a minute')
sleep(60)

print('Blocks seen by nodes:')
for node in chain:
    h, f = node.highest_block()
    print(f'highest:{h} finalized:{f}')

print('Exiting script, leaving nodes running in the background')
