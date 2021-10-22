import logging
import os
import re
import shutil
from argparse import Namespace
from pathlib import Path

from shell import terminate_instances_in_region
from utils import default_region


def stop_protocol(tag: str):
    logging.info('Stopping instances...')
    terminate_instances_in_region(default_region(), tag)
    logging.info('Instances stopped.')


def remove_file(filename: str):
    Path(filename).unlink(missing_ok=True)


def remove_files():
    for filename in ['addresses', 'aleph-node.zip', 'chainspec.json', 'libp2p_public_keys',
                     'validator_accounts', 'validator_phrases', 'x']:
        remove_file(filename)

    shutil.rmtree('accounts', ignore_errors=True)
    shutil.rmtree('bin', ignore_errors=True)
    shutil.rmtree('data', ignore_errors=True)

    for item in os.listdir(os.curdir):
        if re.match(r'data\d+\.zip', item):
            remove_file(item)


def stop_monitoring():
    os.system('docker-compose down')
    remove_file('prometheus.yml')


def clean(args: Namespace):
    stop_protocol(args.tag)
    remove_files()
    if args.kill_monitoring:
        stop_monitoring()
