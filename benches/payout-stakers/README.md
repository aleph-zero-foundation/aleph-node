# `payout_stakers` bench

This is a performance, e2e test for checking rewards for upper bound limit of nominators.

Run by 
``` 
RUST_LOG=aleph_e2e_client=info,aleph-client=info,aleph_client=info,payout_stakers=info cargo run --release -- --validator-count 10
``` 
It needs local nodes to be run (via `scripts/run_nodes.sh` for example.)
