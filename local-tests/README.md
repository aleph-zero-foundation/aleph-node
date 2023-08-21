This folder contains tools for testing the chain locally in a similar fashion to `run_nodes.sh`

The contents are:

1. `chainrunner` - a Python package for running the chain locally on a single machine. Please check the docstrings inside the python source for API documentation.

2. `run_nodes.py` - an example script showing how to use the `chainrunner` package. It mimics the behavior of `run_nodes.sh`

3. `test_catch_up.py` - a script simulating nodes disconnecting from the chain for couple sessions and then reconnecting.

4. `test_multiple_restarts.py` - a script simulating repeated crash and recovery of a single validator within one session.

5. `rolling_update.ipynb` - interactive testing tool for various scenarios of rolling update of the whole chain. Needs to be run via Jupyter (https://jupyter.org/). Please customize constants in the top cell before executing.

6. `test_major_sync.py` -- a script that runs chain for 2 hours and tries to catch-up with one late joining node to the chain.