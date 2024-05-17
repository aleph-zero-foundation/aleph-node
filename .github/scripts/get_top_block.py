#!/usr/bin/env python3
import sys
from sys import argv
import time

from substrateinterface import SubstrateInterface
connection_attempts = 0

while connection_attempts < 15:
    try:
        chain = SubstrateInterface(url=argv[1])
        number = chain.get_block()['header']['number']
        print(number)
        sys.exit(0)
    except Exception as e:
        print("An error occurred: " + str(e), file=sys.stderr)
        connection_attempts += 1
        time.sleep(60)
raise ConnectionRefusedError

