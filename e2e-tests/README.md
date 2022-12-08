# e2e-tests

This crate contains e2e test scenarios for the aleph-node.

## Running

The most basic way to run (assuming a local node is listening on 9944) is:

```bash
$ NODE=ws://127.0.0.1:9944 cargo test name_of_one_test
```

Note that the particular test cases might require different numbers of launched nodes, validators, or a particular
configuration of the launched nodes, see the documentation for a particular test case for details.

Additional options are passed to the tests via env variables. See `src/config.rs` for docs on available options.
