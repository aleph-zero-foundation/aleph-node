# `payout_stakers` bench

This is a performance, e2e test for checking rewards for upper bound limit of nominators.

## How to run

1. Start local chain, e.g. via `./scripts/run_nodes.sh -v 1 -n 1 --dont-build`, where `-n`  denotes number of RPC nodes to start
2. Set `aleph-client` in `Cargo.toml` to point to commit sha which your local `aleph-node` was built from. E.g. below builds `aleph-client` based on `r-13.4` version:
```
aleph_client = { git = "https://github.com/Cardinal-Cryptography/aleph-node", rev = "b072940af9b7295a141a89b943a29a74bb21c9f8" }
```
3. Run 
``` 
RUST_LOG=payout_stakers=info cargo run --release -- --validator-count 1
``` 
where `1` was number of RPC nodes that will eventually become validators. 


