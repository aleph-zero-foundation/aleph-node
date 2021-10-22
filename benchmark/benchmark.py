#!/usr/bin/env python

import argparse
import logging
import os
from pathlib import Path

import fabfile

from experimenting.clean import clean
from experimenting.run import run

logging.basicConfig(level=logging.INFO)


def get_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description='Run experiment on AWS and visualize metrics')
    subparsers = parser.add_subparsers()

    parser_run = subparsers.add_parser('run')
    parser_run.add_argument('nparties', type=int, help='number of nodes in the experiment')
    parser_run.add_argument('aleph_node_binary', type=Path, help='aleph-node executable')
    parser_run.add_argument('--tag', type=str, help='tag for the machines', default='b2')
    parser_run.add_argument('--unit-creation-delay', type=int, help='corresponding flag to the one for chainspec')
    parser_run.set_defaults(func=run)

    parser_clean = subparsers.add_parser('clean')
    parser_clean.add_argument('--kill-monitoring', '-k', action='store_true',
                              help='whether to stop docker with Prometheus and Grafana')
    parser_clean.add_argument('tag', type=str, help='tag for the machines', default='b2')
    parser_clean.set_defaults(func=clean)

    return parser.parse_args()


def setup_env():
    os.environ['FABFILE_PATH'] = fabfile.__file__


if __name__ == '__main__':
    setup_env()
    args = get_args()
    args.func(args)
