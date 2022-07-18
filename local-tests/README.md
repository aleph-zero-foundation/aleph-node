This folder contains tools for testing the chain locally in a similar fashion to `run_nodes.sh`

The contents are:

1. `chainrunner` - a Python package for running the chain locally on a single machine. Please check the docstrings inside the python source for API documentation.

2. `run_nodes.py` - an example script showing how to use the `chainrunner` package. It mimics the behavior of `run_nodes.sh`

3. `test_update.py` - a script simulating an update of `aleph-node` binary together with updating chain's runtime. It requires two different `aleph-node` binaries (pre-update and post-update) and a compiled wasm runtime of the post-update binary. Please check the top part to see how to set up env variables with correct paths before running.

4. `test_catch_up.py` - a script simulating nodes disconnecting from the chain for couple sessions and then reconnecting.

5. `test_multiple_restarts.py` - a script simulating repeated crash and recovery of a single validator within one session.