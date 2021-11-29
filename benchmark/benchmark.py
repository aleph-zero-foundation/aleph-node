#!/usr/bin/env python

import argparse
import logging
import os
from pathlib import Path

import fabfile

from experimenting.clean import clean
from experimenting.run import run
from experimenting.flooder import flood, clean as flood_clean

logging.basicConfig(level=logging.INFO)


def get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description='Run experiment on AWS and visualize metrics')
    subparsers = parser.add_subparsers()

    parser_run = subparsers.add_parser('run')
    parser_run.add_argument('nparties', type=int,
                            help='number of nodes in the experiment')
    parser_run.add_argument(
        'regions', type=str, help='comma separated list of regions where the machines should be created')
    parser_run.add_argument('aleph_node_binary', type=Path,
                            help='aleph-node executable')
    parser_run.add_argument(
        '--tag', type=str, help='tag for the machines', default='b2')
    parser_run.add_argument(
        '--instance', type=str, help='instance type', default='t2.micro')
    parser_run.add_argument('--unit-creation-delay', type=int,
                            help='corresponding flag to the one for aleph-node')
    parser_run.set_defaults(func=run)

    parser_clean = subparsers.add_parser('clean')
    parser_clean.add_argument('--kill-monitoring', '-k', action='store_true',
                              help='whether to stop docker with Prometheus and Grafana')
    parser_clean.add_argument(
        'tag', type=str, help='tag for the machines', default='b2')
    parser_clean.set_defaults(func=clean)

    parser_flooder = subparsers.add_parser('flood')
    parser_flooder.add_argument(
        '--flooder-binary', type=Path, help='flooder executable')
    group = parser_flooder.add_mutually_exclusive_group()
    group.add_argument('--phrase', type=str,
                       help='secret phrase of the account')
    group.add_argument('--seed', type=str, help='secret seed of the account')
    parser_flooder.add_argument(
        '--addresses', type=Path, help='File with URL address(es) of the nodes to send transactions to')
    parser_flooder.add_argument(
        '--transactions', '--tx', type=int, help='how many transactions to send')
    parser_flooder.add_argument(
        '--throughput', type=int, help='what throughput to use (transactions/s)', default=1000)
    parser_flooder.add_argument(
        '--tag', type=str, help='tag for the machines', default='flooders')
    parser_flooder.set_defaults(func=flood)

    parser_flooder_clean = subparsers.add_parser('flooder-clean')
    parser_flooder_clean.add_argument(
        '--tag', type=str, help='tag for the machines', default='flooders')
    parser_flooder_clean.set_defaults(func=flood_clean)

    return parser.parse_args()


def setup_env():
    os.environ['FABFILE_PATH'] = fabfile.__file__


if __name__ == '__main__':
    setup_env()
    args = get_args()
    args.func(args)
