import os
import os.path as op
import subprocess

from .node import Node
from .utils import flags_from_dict, check_file


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
        self.path = op.abspath(workdir)
        self.nodes = []

    def __getitem__(self, i):
        return self.nodes[i]

    def __iter__(self):
        return iter(self.nodes)

    def bootstrap(self, binary, accounts, **kwargs):
        """Bootstrap the chain. `accounts` should be a list of strings.
        Flags `--account-ids`, `--base-path` and `--raw` are added automatically.
        All other flags are taken from kwargs"""
        cmd = [check_file(binary),
               'bootstrap-chain',
               '--base-path', self.path,
               '--account-ids', ','.join(accounts), '--raw']
        cmd += flags_from_dict(kwargs)

        chainspec = op.join(self.path, 'chainspec.json')
        with open(chainspec, 'w', encoding='utf-8') as f:
            subprocess.run(cmd, stdout=f, check=True)

        self.nodes = []
        for a in accounts:
            n = Node(binary, chainspec, op.join(self.path, a), self.path)
            n.flags['node-key-file'] = op.join(self.path, a, 'p2p_secret')
            self.nodes.append(n)

    def set_flags(self, *args, **kwargs):
        """Set common flags for all nodes.
        Positional arguments are used as binary flags and should be strings.
        Keyword arguments are translated to valued flags: `my_arg=some_val` results in
        `--my-arg some_val` in the binary call.
        Seq (type alias for int) can be used to specify numerical values that should be different
        for each node. `val=Seq(13)` results in `--val 13` for node0, `--val 14` for node1 and so
        on."""
        for k in args:
            for n in self.nodes:
                n.flags[k] = True
        for k, v in kwargs.items():
            for i, n in enumerate(self.nodes):
                n.flags[k] = v + i if isinstance(v, Seq) else v

    def set_binary(self, binary, nodes=None):
        """Replace nodes' binary with `binary`. Optional `nodes` argument can be used to specify
        which nodes are affected and should be a list of integer indices (0..N-1).
        Affects all nodes if omitted."""
        check_file(binary)
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

    def start(self, name, nodes=None):
        """Start the chain. `name` will be used to name logfiles: name0.log, name1.log etc.
        Optional `nodes` argument can be used to specify which nodes are affected and should be
        a list of integer indices (0..N-1). Affects all nodes if omitted."""
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].start(name + str(i))

    def stop(self, nodes=None):
        """Stop the chain. Optional `nodes` argument can be used to specify which nodes are affected
        and should be a list of integer indices (0..N-1). Affects all nodes if omitted."""
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].stop()

    def purge(self, nodes=None):
        """Delete the database of the chosen nodes. Optional `nodes` argument can be used to specify
         which nodes are affected and should be a list of integer indices (0..N-1).
         Affects all nodes if omitted."""
        idx = nodes or range(len(self.nodes))
        for i in idx:
            self.nodes[i].purge()
