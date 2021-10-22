import logging
import os
import stat
import webbrowser
from argparse import Namespace
from pathlib import Path
from shutil import copyfile
from time import sleep
from typing import List, Optional

import yaml
from shell import setup_benchmark, instances_ip_in_region, run_task
from utils import default_region


def run_experiment(nparties: int, tag: str, unit_creation_delay: Optional[int]) -> List[str]:
    logging.info('Setting up nodes...')
    flags = {'--unit-creation-delay': unit_creation_delay} if unit_creation_delay else dict()
    setup_benchmark(nparties, 'test', [default_region()], tag=tag, **flags)
    logging.info('Obtaining machine IPs...')
    ips = instances_ip_in_region(tag=tag)
    logging.info(f'Machine IPs: {ips}.')
    logging.info('Dispatching the task...')
    run_task('dispatch', regions=[default_region()], tag=tag)

    logging.info('Running experiment succeeded.')
    return ips


def convert_to_targets(ips: List[str]) -> List[str]:
    return [f'{ip}:9615' for ip in ips]


def create_prometheus_configuration(targets: List[str]):
    logging.info('Creating Prometheus configuration...')

    config = {'scrape_configs': [{
        'job_name': 'aleph-nodes',
        'scrape_interval': '5s',
        'static_configs': [{'targets': targets}]
    }]}

    with open('prometheus.yml', 'w') as yml_file:
        yaml.dump(config, yml_file)

    logging.info('Prometheus configuration saved to `prometheus.yml`.')


def copy_binary(aleph_node_binary: Path):
    logging.info(f'Copying aleph-node binary from {aleph_node_binary}...')

    os.makedirs('bin', exist_ok=True)
    target = Path('bin/aleph-node')
    copyfile(aleph_node_binary, target)
    st = os.stat(target)
    os.chmod(target, st.st_mode | stat.S_IEXEC)

    logging.info(f'Copying aleph-node binary succeeded.')


def run_monitoring_in_docker():
    os.system('docker-compose up -d')


def view_dashboard():
    sleep(2.)  # sometimes the browser is open before Grafana server is up
    webbrowser.open('http://localhost:3000/', 2)


def run(args: Namespace):
    copy_binary(args.aleph_node_binary)
    ips = run_experiment(args.nparties, args.tag, args.unit_creation_delay)
    targets = convert_to_targets(ips)
    create_prometheus_configuration(targets)
    run_monitoring_in_docker()
    view_dashboard()
