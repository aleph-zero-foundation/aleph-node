#!/usr/bin/python3

import argparse
import random
import subprocess
import json
from tabulate import tabulate
import urllib.request

AZERO = 1_000_000_000_000


parser = argparse.ArgumentParser(
    description='Check the prices of some common contract operations')
parser.add_argument('--url', type=str,
                    default='ws://localhost:9944', help='URL of the node to connect to')
parser.add_argument('--suri', type=str, default='//Alice',
                    help='Secret key URI to use for calls')
parser.add_argument('--adder-dir', type=str,
                    help='Directory of the adder contract', default='../../contracts/adder')

args = parser.parse_args()

COMMON_ARGS = ['--suri', args.suri, '--url',
               args.url, '--skip-confirm', '--output-json']


def random_salt():
    return ''.join(random.choice('0123456789abcdef') for _ in range(10))


def deploy(directory):
    res = subprocess.check_output(['cargo', 'contract', 'instantiate', '--salt',
                                   random_salt()] + COMMON_ARGS, cwd=directory)
    return json.loads(res.decode('utf-8'))


def call(directory, contract, message, *args):
    args = [x for a in args for x in ['--args', a]]
    res = subprocess.check_output(['cargo', 'contract', 'call', '--contract', contract,
                                   '--message', message] + args + COMMON_ARGS, cwd=directory)
    return json.loads(res.decode('utf-8'))


def event_field(event, field):
    for f in event['fields']:
        if f['name'] == field:
            return f['value']


def deployer_account_id(deploy_result):
    setup_event = next(filter(
        lambda e: e['name'] == 'Transfer' and account_id(event_field(e, 'to')) == adder_address, deploy_result['events']), None)

    return account_id(event_field(setup_event, 'from'))


def account_id(value):
    match value:
        case {'Literal': account_id}: return account_id
        case _: raise ValueError(f'Invalid account id: {value}')


def uint(value):
    match value:
        case {'UInt': value}: return value
        case _: raise ValueError(f'Invalid uint: {value}')


def find_fee(events, by_whom):
    fee_event = next(filter(lambda e: e['name'] == 'TransactionFeePaid' and account_id(
        event_field(e, 'who')) == by_whom, events), None)
    return uint(event_field(fee_event, 'actual_fee'))


with urllib.request.urlopen('https://api.coingecko.com/api/v3/simple/price?ids=aleph-zero&vs_currencies=usd') as response:
    data = json.load(response)
    aleph_usd = data['aleph-zero']['usd']


def format_fee(fee):
    return "%f AZERO ($%f)" % (fee / AZERO, fee / AZERO * aleph_usd)


deploy_result = deploy(args.adder_dir)

adder_address = deploy_result['contract']
suri_address = deployer_account_id(deploy_result)
instantiate_fee = find_fee(deploy_result['events'], suri_address)

events = call(args.adder_dir, adder_address, 'add', '42')
add_fee = find_fee(events, suri_address)

headers = ['Operation', 'Fee']
prices = [
    ["Instantiate contract with single storage value",
        format_fee(instantiate_fee)],
    ["Call contract with single storage update", format_fee(add_fee)]
]

print(tabulate(prices, headers=headers, tablefmt="github"))
