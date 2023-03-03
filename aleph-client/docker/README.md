This directory contains following files:
### `subxt-integration.Dockerfile`
This is not a main `aleph-client`, rather it is a helper Dockerfile to run on GH, which has `subxt` tool. 

It requires:
* an `aleph-node` chain to be run in the background (ie `127.0.0.1:9944` port must be opened),
* access to `rustfmt.toml`,
* access to current `aleph_zero.rs` file

The docker checks whether a `subxt`-generated runtime metadata is the same as from the current commit. 

It needs to be run only from `aleph-client` directory and in network host mode:
```bash
 docker run --network host --mount type=bind,source="$(pwd)/..",target=/subxt/aleph-node subxt:latest
```

### `subxt-integration-entrypoint.sh` 
An entrypoint for above Dockerfile
