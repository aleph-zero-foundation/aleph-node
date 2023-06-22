# synthetic-network

This folder contains various scripts that allows to spawn and interact with `aleph-node` executed within a so called
synthetic-network. synthetic-network is a tool for docker that allows you to simulate different network conditions, like
variable latency, rate limit, etc. Easiest way to manage parameters of a synthetic-network is to use its web-ui - after
executing `run_consensus_synthetic-network.sh` it should be available at http://localhost:3000 (each node has separate settings
page, i.e. :3001 is Node1, ...).

# Content of this folder

Main file in this folder is `run_consensus_synthetic-network.sh`. It builds a docker-image containing `aleph-node` and some
arbitrary set of networking and debugging tools. It also consists of files required to spawn an instance of the
synthetic-network. Its requirements are: docker, docker-compose, git, `aleph-node:latest` docker-image.

`set_defaults_synthetic-network.sh` allows you to reset settings of the synthetic-network to some sane defaults. You might need
to use it when you set too restrictive values for some of its parameters, i.e. rate limit that make you unable to further
interact with its web-ui.

Additionally, this folder contains an example .js script introducing API of the synthetic-network. You can use it by executing
`run_script_for_synthetic-network.sh --script-path ./latency.js`.

# How to run e2e-tests that use synthetic-network

All following commands are run from within root folder of this repository.

```shell
# build aleph-node docker-image
# it assumes that aleph-node binary is stored at ./target/release/aleph-node
docker build -t aleph-node:latest -f docker/Dockerfile .

# run synthetic-network with aleph-node using docker-compose
# by default, it should build for you a docker-image for synthetic-network
# consult its help for available options
./scripts/synthetic-network/run_consensus_synthetic-network.sh

# run e2e-tests
cd e2e-tests
cargo test --release --no-run --locked
# copy created binary to e2e-tests/target/release/, built test binary is in the last line of
# the above command, e.g.
# cp target/release/deps/aleph_e2e_client-44dc7cbed6112daa target/release/aleph-e2e-client
docker build --tag aleph-e2e-client:latest -f Dockerfile .
cd ..
.github/scripts/run_e2e_test.sh -t high_out_latency_for_all -m 5
```

There's a `OUT_LATENCY` env which control output latency in e2e tests. If not specified, there's 200ms
default used:
```shell
OUT_LATENCY=300 .github/scripts/run_e2e_test.sh -t high_out_latency_for_all -m 5
```

If you'd like to start `run_consensus_synthetic-network.sh` again, run below command first. 
That will clear down docker storage, in particular it will clear previous latency setting.
```shell
docker-compose -f docker/docker-compose.synthetic-network.yml down
```
