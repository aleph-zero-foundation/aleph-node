# e2e-tests

This crate contains e2e test scenarios for the aleph-node.

## Running

The most basic way to run (assuming a local node is listening on 9944) is:

```bash
$ NODE_URL=ws://127.0.0.1:9944 cargo test name_of_one_test
```

Note that the particular test cases might require different numbers of launched nodes, validators, or a particular
configuration of the launched nodes, see the documentation for a particular test case for details.

Additional options are passed to the tests via env variables. See `src/config.rs` for docs on available options.

## Running e2e-tests that depend on synthetic-network, e.g. `sync`, `high_latency`

These tests require the synthetic-network component to work properly. synthetic-network is a docker
based application that allows to manipulate network conditions between nodes, e.g. latency,
bit-rate, etc. You can run these tests by building a custom docker image containing aleph-node with
synthetic-network and executing e2e-tests within a docker container. Example:

```bash
# build aleph-node docker-image
# we assume that aleph-node binary is stored at ./target/release/aleph-node
aleph-node$ docker build -t aleph-node:latest -f docker/Dockerfile .

# build e2e-tests
aleph-node$ cd e2e-tests
e2e-tests$ cargo test --release --no-run --locked
# copy created binary to e2e-tests/target/release/, built test binary is in the last line of
# the above command, e.g.
# cp target/release/deps/aleph_e2e_client-44dc7cbed6112daa target/release/aleph-e2e-client
e2e-tests$ docker build --tag aleph-e2e-client:latest -f Dockerfile .
e2e-tests$ cd ..

# run synthetic-network with aleph-node using docker-compose
# by default, it should build for you a docker-image for synthetic-network
aleph-node$ NODES_COUNT=7 DOCKER_COMPOSE=./docker/docker-compose.synthetic-network_sync-tests.yml ./scripts/synthetic-network/run_consensus_synthetic-network.sh

# run e2e-tests
# run tests for the block sync component
aleph-node$ VALIDATOR_COUNT=7 NETWORK="synthetic-network" NODE_URL="ws://Node0:9944" ./.github/scripts/run_e2e_test.sh -t test::sync -m 7
# run high-latency tests
aleph-node$ OUT_LATENCY=500 ./.github/scripts/run_e2e_test.sh -t high_out_latency_for_all -m 5
```

## Running on devnet (or other-net)

You can also run the tests on some other network. For example, to run the contract test for the `adder` contract on
devnet:

1. Prepare an account with some money, note the seed of the account.
2. Deploy the contract to devnet:

```bash
contracts/adder$ NODE_URL=wss://ws.dev.azero.dev AUTHORITY="$THE_SEED" ./deploy.sh
```

3. Run the tests:

```bash
e2e-tests$ RUST_BACKTRACE=1 SUDO_SEED="$THE_SEED" NODE_URL=wss://ws.dev.azero.dev:443 \
  ADDER=$DEPLOY_ADDRESS ADDER_METADATA=../contracts/adder/target/ink/metadata.json cargo test adder -- --nocapture
```

## Running on feature net

Run a feature net by adding an appropriate label to a pull request, ie `trigger:create-featurenet`
, then after its started run

```bash
e2e-tests$ RUST_LOG=info NODE_URL=wss://ws-fe-a0-29025887979136.dev.azero.dev:443 cargo test --release finalization::finalization -- --nocapture
```

where you can find your feature env address in https://github.com/Cardinal-Cryptography/aleph-node/deployments

In you have docker image of `e2e-client`, you can run above test with command
```bash
docker run --network host -e NODE_URL="wss://ws-fe-a0-29025887979136.dev.azero.dev:443" -e TEST_CASES="finalization::finalization" -e RUST_LOG=info  aleph-e2e-client:latest
```
