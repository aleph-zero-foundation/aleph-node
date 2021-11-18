import shutil

from argparse import Namespace
from pathlib import Path
from typing import List, Optional
from shell import setup_flooding, terminate_instances_in_region

from .utils import copy_binary


def generate_script(
    target: str,
    nodes: List[str],
    seed: Optional[str],
    phrase: Optional[str],
    transactions: int,
    throughput: int
):
    shebang = '#!/usr/bin/env bash\n'

    cmd = f'RUST_LOG=info ./flooder --nodes {" ".join([f"{ip}:9944" for ip in nodes])} --transactions={transactions} --throughput={throughput}'

    if seed is not None:
        cmd = f'{cmd} --seed={seed}\n'
    
    if phrase is not None:
        cmd = f'{cmd} --phrase="{phrase}"\n'

    with open(target, 'w') as f:
        f.writelines([shebang, cmd])


def parse_addresses(addresses: Path):
    with open(addresses, 'r') as f:
        return [ip.strip() for ip in f.readlines()]

    return []


def clean_files():
    shutil.rmtree('bin/flooder_script.sh', ignore_errors=True)
    shutil.rmtree('bin/flooder', ignore_errors=True)


def clean(args: Namespace):
    clean_files()
    terminate_instances_in_region(tag=args.tag)


def flood(args: Namespace):
    copy_binary(args.flooder_binary, 'flooder')

    generate_script(
        target='bin/flooder_script.sh',
        nodes=parse_addresses(args.addresses),
        seed=args.seed,
        phrase=args.phrase,
        transactions=args.transactions,
        throughput=args.throughput
    )

    setup_flooding(tag=args.tag)
