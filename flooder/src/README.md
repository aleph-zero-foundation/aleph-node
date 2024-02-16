# flooder

Tool for flooding nodes with transactions.
Makes several connections with nodes 
and submits `transactions-in-interval` transactions
every `interval-length` seconds.
## Running
Assuming you have locally running network (e.g. via `run_nodes.sh`),
and test account, determined by either `--phrase or --seed`, has enough funds
for the test:
```bash
cargo run --release -- --transactions-in-interval 100 --intervals 100
```