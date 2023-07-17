import jsonrpcclient
import os.path as op
import re
import subprocess
from collections import OrderedDict


def generate_keys(binary, phrases):
    """Generate public keys based on the list of seed phrases.
    `binary` should be a path to some `aleph-node` binary. `phrases` should be a list of strings.
    Returns an ordered dictionary with phrases as keys and corresponding public keys as values.
    The order follows the order in `phrases`.
    """
    binary = check_file(binary)
    regexp = re.compile(r'SS58 Address:\s*(\w+)$', re.MULTILINE)
    res = OrderedDict()
    for p in phrases:
        out = subprocess.check_output([binary, 'key', 'inspect', p]).decode()
        matches = regexp.findall(out)
        res[p] = matches[0] if matches else None
    return res


def check_file(path):
    """Ensure the provided path points to an existing file."""
    path = op.expandvars(path)
    if not op.isfile(path):
        raise FileNotFoundError(f'file not found: {path}')
    return path


def flag(s):
    """Turn 'flag_name' into `--flag-name`."""
    return '--' + str(s).replace('_', '-')


def flags_from_dict(d):
    """Turn a dictionary of flags into a list of strings required by subprocess methods."""
    res = []
    for k, v in d.items():
        res.append(flag(k))
        if v is not True:
            val = str(v)
            if ' ' in val:
                res += val.split(' ')
            else:
                res.append(val)
    return res


def check_finalized(nodes):
    """Check nodes stats, print them and return finalized block number per node"""
    results = [node.highest_block() for node in nodes]
    highest, finalized = zip(*results)
    print('Blocks seen:')
    print('  Highest:   ', *highest)
    print('  Finalized: ', *finalized)
    return finalized


def check_version(nodes, verbose=False):
    """Query given nodes for aleph-node (host) version and runtime version.
    Print the summary to the standard output and return the runtime version.
    If multiple runtime versions are reported, print error and return the maximum.
    If `verbose` is True, print the whole RPC response."""
    versions = set()
    print('Node versions:')
    for i, node in enumerate(nodes):
        sysver = node.rpc('system_version').result
        resp = node.rpc('state_getRuntimeVersion')
        if verbose:
            print(resp)
        if isinstance(resp, jsonrpcclient.Ok):
            rt = resp.result['specVersion']
            versions.add(rt)
        else:
            rt = "ERROR"
        print(f'  Node {i} | host: {sysver}  runtime: {rt}')
    if len(versions) > 1:
        print(f'ERROR: nodes reported different runtime versions: {versions}')
    if versions:
        return max(versions)
    return -1
