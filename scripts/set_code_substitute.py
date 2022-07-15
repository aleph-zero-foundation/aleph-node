#!/bin/python3

import argparse
import json
import logging
from pathlib import Path

logging.basicConfig(level=logging.INFO)


def get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description='Inject wasm blob as code-substitute into chainspec')

    parser.add_argument('old_chainspec', type=Path,
                        help='Path to the old chainspec (the one to be extended)')
    parser.add_argument('new_chainspec', type=Path,
                        help='New chainspec path (the one to be created)')
    parser.add_argument('block_number', type=int,
                        help='From what height the new runtime should be substituted')
    parser.add_argument('runtime', type=Path, help='Path to the substituting runtime')

    return parser.parse_args()


def update_chainspec(old_chainspec: Path, new_chainspec: Path, block_number: int, runtime: Path):
    logging.info(f'Setting `code_substitute` from block #{block_number}.')

    with open(old_chainspec, mode='r', encoding='utf-8') as chainspec_in:
        chainspec = json.loads(chainspec_in.read())
    logging.info(f'✅ Read old chainspec from {old_chainspec}')

    with open(runtime, mode='rb') as substitute:
        substitute = substitute.read().hex()
    logging.info(f'✅ Read runtime from {runtime}')

    chainspec['codeSubstitutes'] = {block_number: f'0x{substitute}'}

    with open(new_chainspec, mode='w', encoding='utf-8') as chainspec_out:
        chainspec_out.write(json.dumps(chainspec, indent=2))
    logging.info(f'✅ Saved new chainspec to {new_chainspec}')


def check_files(files: [Path]):
    for file in files:
        assert file.is_file(), f'❌ File `{file}` was not found'


if __name__ == '__main__':
    args = get_args()
    old_chainspec, new_chainspec, block_number, runtime = \
        args.old_chainspec, args.new_chainspec, args.block_number, args.runtime

    check_files([old_chainspec, runtime])

    update_chainspec(old_chainspec, new_chainspec, block_number, runtime)
