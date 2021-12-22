This folder contains tools for testing the chain locally in a similar fashion to `run_nodes.sh`

The contents are:

1. `send-runtime` - a tiny Rust binary for sending a `setCode` extrinsic to a running node. It depends on `substrate-api-client` so it should be compiled with a nightly toolchain. It's called with 3 arguments: a node WS address, a sudo account seed phrase and a path to a wasm runtime file (compiled with `cargo build --release -p aleph-runtime` and taken from `target/release/wbuild/aleph-runtime/aleph_runtime.compact.wasm`).

2. `chainrunner` - a Python package for running the chain locally on a single machine. Please check the docstrings inside the python source for API documentation.

3. `run_nodes.py` - an example script showing how to use the `chainrunner` package. It mimics the behavior of `run_nodes.sh`

4. `test_update.py` - a script simulating an update of `aleph-node` binary together with updating chain's runtime. It requires two different `aleph-node` binaries (pre-update and post-update) and a compiled wasm runtime of the post-update binary. Please edit the top part and enter the correct paths before running.

5. `test_update.ipynb` - a Jupyter notebook with the same contents as `test_update.py`