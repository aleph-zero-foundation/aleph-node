#!/bin/env python
import argparse
import json
import logging
import os
import subprocess
from pathlib import Path

from chainrunner import Chain, Seq, generate_keys

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s %(levelname)-8s %(message)s',
)

WORKDIR = os.path.abspath(os.getenv('WORKDIR', '/tmp'))


def file(filepath: str) -> Path:
    logging.debug(f'Looking for file {filepath}...')
    path = Path(filepath)
    if not path.is_file():
        raise argparse.ArgumentTypeError(f'❌ File `{filepath}` was not found')
    return path


def get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description='Test runtime update with `try-runtime`')

    parser.add_argument('live_chain', type=str, help='Address to the live chain')
    parser.add_argument('chainspec', type=file, help='Path to the original raw chainspec')
    parser.add_argument('new_runtime', type=file, help='Path to the new runtime')
    parser.add_argument('try_runtime', type=file, help='Path to the `try-runtime` tool')

    return parser.parse_args()


def save_runtime_to_chainspec(chainspec_path: Path, new_chainspec_path: Path, runtime_path: Path):
    logging.info(f'Setting code to the content of {runtime_path}...')

    with open(chainspec_path, mode='r', encoding='utf-8') as chainspec_file:
        chainspec = json.loads(chainspec_file.read())
    logging.debug(f'✅ Read chainspec from {chainspec_path}')

    with open(runtime_path, mode='rb') as runtime_file:
        runtime = runtime_file.read().hex()
    logging.debug(f'✅ Read runtime from {runtime_path}')

    chainspec['genesis']['raw']['top']['0x3a636f6465'] = f'0x{runtime}'

    with open(new_chainspec_path, mode='w', encoding='utf-8') as chainspec_file:
        chainspec_file.write(json.dumps(chainspec, indent=2))
    logging.info(f'✅ Saved updated chainspec to {new_chainspec_path}')


def test_update(try_runtime: Path, chainspec: Path, address: str):
    cmd = [try_runtime, 'try-runtime', '--chain', chainspec, 'on-runtime-upgrade', 'live',
           '--uri', address]
    logging.info('Running `try-runtime` check...')
    subprocess.run(cmd, check=True)
    logging.info('✅ Update has been successful!')


def run_test(live_chain: str, chainspec: Path, new_runtime: Path, try_runtime: Path):
    new_chainspec = Path(os.path.join(WORKDIR, 'chainspec.json'))
    save_runtime_to_chainspec(chainspec, new_chainspec, new_runtime)
    test_update(try_runtime, new_chainspec, live_chain)


if __name__ == '__main__':
    args = get_args()
    run_test(args.live_chain, args.chainspec, args.new_runtime, args.try_runtime)
