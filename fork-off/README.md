## Aleph Fork-Off tool

Command-line tool for creating snapshots of a running aleph-bft chains.
Given a (raw) chainspec of the target chain and an url to a node of the said chain to query it will create a raw chainspec, with the genesis block equal to the current state of the target chain.

In addition, there is a possibility of tweaking initial balances for specified accounts.

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

The tool will perform the following actions, in this order:
1. Download the whole state (key-value pairs) of the chain via the provided rpc endpoint `ws-rpc-endpoint`. More specifically it will first query the best block and then download the state at this block.
2. Dump the state to a json file. You can provide a path via `--snapshot-path`.
3. Read the state from the snapshot json file. This is because steps 1. and 2. can be omitted by running with `--use-snapshot-file` -- see example below.
4. Read the chainspec provided via `--initial-spec-path` you should pass here the one generated via `the bootstrap-chain` command, so `--initial-spec-path=chainspec.json` if it is in the same directory.
5. Replace the genesis state in the chainspec by the one from the snapshot WITH THE EXCEPTION of states of paths provided via a comma separated list using `--storage_keep_state`. The default setting is `--storage_keep_state=Aura,Aleph,Sudo,Staking,Session,Elections` and it's likely you don't want to change it.
6. If you have passed `--accounts_path` pointing to a file with a configuration for some accounts, then it will be written into chainspec (`System.Account` map). For an example see `AccountInfo-Template.json` file.
7. Alternatively to `--accounts_path` you can just pass `--balances` flag with which you can specify initial free balances for some accounts. Similarly, it will be saved to `System.Account` map.
8. The final, new chainspec is saved to the path provided via `--combined-spec-path`.

Note: `fork-off` expects address as an SS58 public key. 
For Alice it would be `5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY`.

So for instance to generate a new spec keeping the storage of testnet (note that in that case you should use the same binary as running on testnet to `bootstrap-chain`) we would run:

```bash
target/release/fork-off --ws-rpc-endpoint=wss://ws.test.azero.dev --initial-spec-path=chainspec.json --combined-spec-path=combined.json --balances 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY=1000000000000000
```

This will also create a `snapshot.json` file containing the state downloaded from testnet. In case the state downloaded correctly (easy to see from logs) but something went wrong when combining the specs (e.g. you want to use a different set of paths) then you can rerun without the need of downloading the state again (it might be time consuming):

```bash
target/release/fork-off --ws-rpc-endpoint=wss://ws.test.azero.dev --initial-spec-path=chainspec.json --combined-spec-path=combined.json --use-snapshot-file
```

Finally, there is also an optional parameter `--max-requests` with a default value of `1000` which you can tweak to allow more/less concurrent in-flight requests while the state is downloading. Note that this might influence the risk of being banned for too many RPC requests, so use with caution. The default value seems to be safe.
