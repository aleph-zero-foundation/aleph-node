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

See [readme](../scripts/synthetic-network/README.md).

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
