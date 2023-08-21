import os
import subprocess
import time
from os.path import join, abspath

from .node import Node
from .utils import flags_from_dict, check_file, check_finalized, check_version


# Seq is a wrapper type around int for supplying numerical parameters
# that should differ for each node (ports etc.)
class Seq(int):
    pass


class Chain:
    """Chain is a class for orchestrating setting up and interaction with a local aleph-node
    blockchain. The constructor takes only one argument - a path to a directory with the workspace.
    All other parameters are adjusted with `bootstrap()` and `set_flags()`."""

    def __init__(self, workdir):
        os.makedirs(workdir, exist_ok=True)
        self.path = abspath(workdir)
        self.nodes = []
        self.validator_nodes = []
        self.nonvalidator_nodes = []

    def __getitem__(self, i):
        return self.nodes[i]

    def __iter__(self):
        return iter(self.nodes)

    def new(self, binary, validators, nonvalidators=None):
        """Initialize the chain but does not do any bootstraping. `validators` and `nonvalidators` should be lists
        of strings with public keys."""
        nonvalidators = nonvalidators or []
        binary = check_file(binary)
        chainspec = join(self.path, 'chainspec.json')

        self.validator_nodes = [Node(i, binary, chainspec, join(self.path, v), self.path) for (i,v) in enumerate(validators)]
        self.nonvalidator_nodes = [Node(i+len(validators), binary, chainspec, join(self.path, nv), self.path) for (i,nv) in enumerate(nonvalidators)]

        self.nodes = self.validator_nodes + self.nonvalidator_nodes

    def bootstrap(self, binary, validators, nonvalidators=None, raw=True, **kwargs):
        """Bootstrap the chain. `validators` and `nonvalidators` should be lists of strings
        with public keys. Flags `--account-ids`, `--base-path` and `--raw` are added automatically.
        All other flags are taken from kwargs"""
        self.new(binary, validators, nonvalidators)
        nonvalidators = nonvalidators or []

        binary = check_file(binary)
        cmd = [binary, 'bootstrap-chain',
               '--base-path', self.path,
               '--account-ids', ','.join(validators)]
        if raw:
            cmd.append('--raw')
        cmd += flags_from_dict(kwargs)

        chainspec = join(self.path, 'chainspec.json')
        with open(chainspec, 'w', encoding='utf-8') as f:
            subprocess.run(cmd, stdout=f, check=True)

        for nv in nonvalidators:
            cmd = [binary, 'bootstrap-node',
                   '--base-path', join(self.path, nv),
                   '--account-id', nv]
            subprocess.run(cmd, stdout=subprocess.DEVNULL, check=True)

    @staticmethod
    def _set_flags(nodes, *args, **kwargs):
        for k in args:
            for n in nodes:
                n.flags[k] = True
        for k, v in kwargs.items():
            for i, n in enumerate(nodes):
                if isinstance(v, Seq):
                    n.flags[k] = v + i
                elif isinstance(v, list) and i < len(v):
                    n.flags[k] = v[i]
                else:
                    n.flags[k] = v

    def set_flags(self, *args, **kwargs):
        """Set common flags for all nodes.
        Positional arguments are used as binary flags and should be strings.
        Keyword arguments are translated to valued flags: `my_arg=some_val` results in
        `--my-arg some_val` in the binary call.
        Seq (type alias for int) can be used to specify numerical values that should be different
        for each node. `val=Seq(13)` results in `--val 13` for node0, `--val 14` for node1 and so
        on.
        Providing a list of values results in each node being assigned a corresponding value from the list."""
        Chain._set_flags(self.nodes, *args, **kwargs)

    def set_flags_validator(self, *args, **kwargs):
        """Set common flags for all validator nodes.
        Positional arguments are used as binary flags and should be strings.
        Keyword arguments are translated to valued flags: `my_arg=some_val` results in
        `--my-arg some_val` in the binary call.
        Seq (type alias for int) can be used to specify numerical values that should be different
        for each node. `val=Seq(13)` results in `--val 13` for node0, `--val 14` for node1 and so
        on.
        Providing a list of values results in each node being assigned a corresponding value from the list."""
        Chain._set_flags(self.validator_nodes, *args, **kwargs)

    def set_flags_nonvalidator(self, *args, **kwargs):
        """Set common flags for all nonvalidator nodes.
        Positional arguments are used as binary flags and should be strings.
        Keyword arguments are translated to valued flags: `my_arg=some_val` results in
        `--my-arg some_val` in the binary call.
        Seq (type alias for int) can be used to specify numerical values that should be different
        for each node. `val=Seq(13)` results in `--val 13` for node0, `--val 14` for node1 and so
        on.
        Providing a list of values results in each node being assigned a corresponding value from the list."""
        Chain._set_flags(self.nonvalidator_nodes, *args, **kwargs)

    def set_binary(self, binary, nodes=None):
        """Replace nodes' binary with `binary`. Optional `nodes` argument can be used to specify
        which nodes are affected and should be a list of integer indices (0..N-1).
        Affects all nodes if omitted."""
        binary = check_file(binary)
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].binary = binary

    def set_chainspec(self, chainspec, nodes=None):
        """Replace nodes' chainspec with `chainspec`. Optional `nodes` argument can be used to
        specify which nodes are affected and should be a list of integer indices (0..N-1).
        Affects all nodes if omitted."""
        check_file(chainspec)
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].chainspec = chainspec

    def set_log_level(self, target, level, nodes=None):
        """Change log verbosity of the chosen logging target. This method works on the fly
        (performs RPCs) and should be called while the chain is running.
        Optional `nodes` argument can be used to specify which nodes are affected and should be
        a list of integer indices (0..N-1). Affects all nodes if omitted."""
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].set_log_level(target, level)

    def start(self, name, nodes=None, backup=True):
        """Start the chain. `name` will be used to name logfiles: name0.log, name1.log etc.
        Optional `nodes` argument can be used to specify which nodes are affected and should be
        a list of integer indices (0..N-1). Affects all nodes if omitted."""
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].start(name, backup)

    def stop(self, nodes=None):
        """Stop the chain. Optional `nodes` argument can be used to specify which nodes are affected
        and should be a list of integer indices (0..N-1). Affects all nodes if omitted."""
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].stop()

    def status(self):
        """Prints to stdout basic status check of the chain: blocks seen by nodes and binaries versions."""
        check_finalized(self)
        check_version(self)

    def purge(self, nodes=None):
        """Delete the database of the chosen nodes. Optional `nodes` argument can be used to specify
         which nodes are affected and should be a list of integer indices (0..N-1).
         Affects all nodes if omitted."""
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].purge()

    def fork(self, forkoff_path, ws_endpoint, snapshot_file=None):
        """Replace the chainspec of this chain with the state forked from the given `ws_endpoint`.
        This method should be run after bootstrapping the chain, but before starting it.
        'forkoff_path' should be a path to fork-off binary."""
        forked = join(self.path, 'forked.json')
        chainspec = join(self.path, 'chainspec.json')
        snapshot = snapshot_file if snapshot_file else join(self.path, 'snapshot.json')
        cmd = [check_file(forkoff_path), '--ws-rpc-endpoint', ws_endpoint,
               '--initial-spec-path', chainspec,
               '--snapshot-path', snapshot,
               '--combined-spec-path', forked]
        if snapshot_file:
            cmd.append('--use-snapshot-file')
        subprocess.run(cmd, check=True)
        self.set_chainspec(forked)

    def get_highest_imported(self, nodes=None):
        """Return the maximum height such that each of the selected nodes (all nodes if None)
        imported a block of such height."""
        nodes = range(len(self.nodes)) if nodes is None else nodes
        nodes = [self.nodes[i] for i in nodes]
        return min([n.highest_block()[0] for n in nodes], default=-1)

    def get_highest_finalized(self, nodes=None):
        """Return the maximum height such that each of the selected nodes (all nodes if None)
        finalized a block of such height."""
        nodes = range(len(self.nodes)) if nodes is None else nodes
        nodes = [self.nodes[i] for i in nodes]
        return min([n.highest_block()[1] for n in nodes], default=-1)

    def wait_for_finalization(self, old_finalized, nodes=None, timeout=600, finalized_delta=3, catchup=True, catchup_delta=10):
        """Wait for finalization to catch up with the newest blocks. Requires providing the number
        of the last finalized block, which will be used as a reference against recently finalized blocks.
        The finalization is considered "recovered" when all provided `nodes` (all nodes if None)
        have seen a finalized block higher than `old_finalized` + `finalized_delta`.
        If `catchup` is True, wait until finalization catches up with the newly produced blocks
        (within `catchup_delta` blocks). 'timeout' (in seconds) is a global timeout for the whole method
        to execute. Raise TimeoutError if finalization fails to recover within the given timeout."""
        nodes = range(len(self.nodes)) if nodes is None else nodes
        deadline = time.time() + timeout
        while self.get_highest_finalized(nodes) <= old_finalized + finalized_delta:
            time.sleep(5)
            if time.time() > deadline:
                raise TimeoutError(f'Block finalization stalled after {timeout} seconds')
        nodes = [self.nodes[i] for i in nodes]
        if catchup:
            def lags(node):
                r, f = node.highest_block()
                return r - f > catchup_delta
            while any(lags(n) for n in nodes):
                time.sleep(5)
                if time.time() > deadline:
                    print(f'Finalization restored, but failed to catch up with recent blocks within {timeout} seconds')
                    break

    def wait_for_imported_at_height(self, height, nodes=None, timeout=600):
        """Wait until all the selected `nodes` (all nodes if None) imported a block at height `height`"""
        nodes = range(len(self.nodes)) if nodes is None else nodes
        deadline = time.time() + timeout
        while self.get_highest_imported(nodes) < height:
            time.sleep(1)
            if time.time() > deadline:
                raise TimeoutError(f'Block production stalled after {timeout} seconds')

    def wait_for_authorities(self, nodes=None, timeout=600):
        """Wait for the selected `nodes` (all validator nodes if None) to connect to all known authorities.
        If not successful within the given `timeout` (in seconds), raise TimeoutError."""
        nodes = [self.nodes[i] for i in nodes] if nodes is not None else self.validator_nodes
        deadline = time.time() + timeout
        while not all(n.check_authorities() for n in nodes):
            time.sleep(5)
            if time.time() > deadline:
                raise TimeoutError(f'Failed to connect to all authorities after {timeout} seconds')
