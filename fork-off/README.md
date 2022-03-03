## Aleph Fork-Off tool

Command-line tool for creating snapshots of a running aleph-bft chains.
Given a (raw) chainspec of the target chain and a url to a node of the said chain to query it will create a raw chainspec, with the genesis block equal to the current state of the target chain.

You can then spawn a forked-off chain using this chainspec as a starting point.

## Using instructions

Build the binary (this command will use rust toolchain version specified in the `rust-toolchain` file
located in this repository's top-level directory):

```bash
cargo build --release
```

Create a chainspec for the fork, it will serve as a basis with a known sudo account, known set of validators, known session keys etc:

```bash
aleph-node bootstrap-chain --raw --base-path /data --chain-id a0fnet1 --account-ids <id1,id2,...>  --sudo-account-id <sudo_id> > chainspec.json
```

Alternatively, if you have a chainspec in a human-readable format, you can convert it into the "raw" format using the `convert-chainspec-to-raw` command:

```bash
aleph-node convert-chainspec-to-raw --chain docker/data/chainspec.json
```

Tool will query the target chain for storage pairs (by default "Aura" and "Aleph") and copy them over to the target fork chainspec, which is finally written out to the specified path:

```bash
RUST_LOG=info target/release/fork-off --http-rpc-endpoint http://127.0.0.1:9933 --fork-spec-path chainspec.json --write-to-path chainspec.fork.json --prefixes <Pallet1,Pallet2,...>
```

where:

* `http-rpc-endpoint`: is an URL address of an RPC endpoint of a target chain node (for querying current state).
* `fork-spec-path`: a path to the generated chainspec, the basis for creating the fork.
* `write-to-path`: where to write the resulting chainspec to.
* `prefixes`: which storage items to migrate ("Aura" and "Aleph" by default), e.g. , `--prefixes Aura,Aleph,Treasury,Vesting`
