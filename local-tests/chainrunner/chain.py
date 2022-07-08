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
        self.validator_nodes = []
        self.nonvalidator_nodes = []

    def __getitem__(self, i):
        return self.nodes[i]

    def __iter__(self):
        return iter(self.nodes)

    def bootstrap(self, binary, validators, nonvalidators=None, raw=True, **kwargs):
        """Bootstrap the chain. `validator_accounts` should be a list of strings.
        Flags `--account-ids`, `--base-path` and `--raw` are added automatically.
        All other flags are taken from kwargs"""
        nonvalidators = nonvalidators or []
        cmd = [check_file(binary),
               'bootstrap-chain',
               '--base-path', self.path,
               '--account-ids', ','.join(validators)]
        if raw:
            cmd.append('--raw')
        cmd += flags_from_dict(kwargs)

        chainspec = op.join(self.path, 'chainspec.json')
        with open(chainspec, 'w', encoding='utf-8') as f:
            subprocess.run(cmd, stdout=f, check=True)

        for nv in nonvalidators:
            cmd = [check_file(binary),
                   'bootstrap-node',
                   '--base-path', self.path,
                   '--account-id', nv]
            subprocess.run(cmd, stdout=subprocess.DEVNULL, check=True)

        def account_to_node(account):
            n = Node(binary, chainspec, op.join(self.path, account), self.path)
            n.flags['node-key-file'] = op.join(self.path, account, 'p2p_secret')
            n.flags['backup_path'] = op.join(self.path, account, 'backup-stash')
            n.flags['enable-log-reloading'] = True
            return n

        self.validator_nodes = [account_to_node(a) for a in validators]
        self.nonvalidator_nodes = [account_to_node(a) for a in nonvalidators]

        self.nodes = self.validator_nodes + self.nonvalidator_nodes

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
