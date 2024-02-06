#!/usr/bin/env python3
from sys import argv
from substrateinterface import SubstrateInterface
chain = SubstrateInterface(url=argv[1])
number = chain.get_block()['header']['number']
print(number)
