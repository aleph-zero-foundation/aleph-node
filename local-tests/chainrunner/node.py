import json
import os.path as op
import re
import subprocess

import jsonrpcclient as rpc
import requests

from .utils import flags_from_dict


class Node:
    """A class representing a single node of a running blockchain.
    `binary` should be a path to a file with aleph-node binary.
    `chainspec` should be a path to a file with chainspec,
    `path` should point to a folder where the node's base path is."""

    def __init__(self, binary, chainspec, path, logdir=None):
        self.chainspec = chainspec
        self.binary = binary
        self.path = path
        self.logdir = logdir or path
        self.logfile = None
        self.process = None
        self.flags = {}
        self.running = False

    def _stdargs(self):
        return ['--base-path', self.path, '--chain', self.chainspec]

    def start(self, name):
        """Start the node. `name` is used to name of the logfile and for --name flag."""
        cmd = [self.binary, '--name', name] + self._stdargs() + flags_from_dict(self.flags)

        self.logfile = op.join(self.logdir, name + '.log')
        with open(self.logfile, 'w', encoding='utf-8') as logfile:
            self.process = subprocess.Popen(cmd, stderr=logfile, stdout=subprocess.DEVNULL)
        self.running = True

    def stop(self):
        """Stop the node by sending SIGKILL."""
        if self.running:
            self.process.kill()
            self.running = False

    def purge(self):
        """Purge chain (delete the database of the node)."""
        cmd = [self.binary, 'purge-chain', '-y'] + self._stdargs()
        subprocess.run(cmd, stdout=subprocess.DEVNULL, check=True)

    def greplog(self, regexp):
        """Find in the logs all occurrences of the given regexp. Returns a list of matches."""
        if not self.logfile:
            return []
        with open(self.logfile, encoding='utf-8') as f:
            log = f.read()
        return re.findall(regexp, log)

    def highest_block(self):
        """Find in the logs the height of the most recent block.
        Returns two ints: highest block and highest finalized block."""
        results = self.greplog(r'best: #(\d+) .+ finalized #(\d+)')
        if results:
            a, b = results[-1]
            return int(a), int(b)
        return -1, -1

    def get_hash(self, height):
        """Find the hash of the block with the given height. Requires the node to be running."""
        return self.rpc('chain_getBlockHash', [height]).result

    def state(self, block=None):
        """Return a JSON representation of the chain state after the given block.
        If `block` is `None`, the most recent state (after the highest seen block) is returned.
        Node must not be running, empty result is returned if called on a running node."""
        if self.running:
            print("cannot export state of a running node")
            return {}
        cmd = [self.binary, 'export-state'] + self._stdargs()
        if block is not None:
            cmd.append(str(block))
        proc = subprocess.run(cmd, capture_output=True, check=True)
        return json.loads(proc.stdout)

    def rpc(self, method, params=None):
        """Make an RPC call to the node with the given method and params.
        `params` should be a tuple for positional arguments, or a dict for keyword arguments."""
        if not self.running:
            print("cannot RPC because node is not running")
            return None
        port = self.flags.get('rpc_port', self.flags.get('rpc-port', -1))
        if port == -1:
            print("RPC port unknown, please set rpc_port flag")
            return None
        resp = requests.post(f'http://localhost:{port}/', json=rpc.request(method, params))
        return rpc.parse(resp.json())

    def set_log_level(self, target, level):
        """Change log verbosity of the chosen target.
        This method should be called on a running node."""
        return self.rpc('system_addLogFilter', [f'{target}={level}'])

    def address(self, port=None):
        """Get the public address of this node. Returned value is of the form
        /dns4/localhost/tcp/{PORT}/p2p/{KEY}. This method needs to know node's port -
        if it's not supplied a as parameter, it must be present in `self.flags`.
        """
        if port is None:
            if 'port' in self.flags:
                port = self.flags['port']
            else:
                return None
        cmd = [self.binary, 'key', 'inspect-node-key', '--file', op.join(self.path, 'p2p_secret')]
        output = subprocess.check_output(cmd).decode().strip()
        return f'/dns4/localhost/tcp/{port}/p2p/{output}'
