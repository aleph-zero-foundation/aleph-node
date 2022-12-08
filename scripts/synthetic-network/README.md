# synthetic-network

This folder contains various scripts that allows to spawn and interact with `aleph-node` executed within a so called
synthetic-network. synthetic-network is a tool for docker that allows you to simulate different network conditions, like
variable latency, rate limit, etc. Easiest way to manage parameters of a synthetic-network is to use its web-ui - after
executing `run_consensus_synthetic-network.sh` it should be available at http://localhost:3000 (each node has separate settings
page, i.e. :3001 is Node1, ...).

# Content of this folder

Main file in this folder is `run_consensus_synthetic-network.sh`. It builds a docker-image containing `aleph-node` and some
arbitrary set of networking and debugging tools. It also consist of files required to spawn an instance of the
synthetic-network. Its requirements are: docker, docker-compose, git, `aleph-node:latest` docker-image.

`set_defaults_synthetic-network.sh` allows you to reset settings of the synthetic-network to some sane defaults. You might need
to use it when you set too restrictive values for some of its parameters, i.e. rate limit that make you unable to further
interact with its web-ui.

Additionally, this folder contains an example .js script introducing API of the synthetic-network. You can use it by executing
`run_script_for_synthetic-network.sh --script-path ./latency.js`.

