This folder contains tools for testing the chain locally in a similar fashion to `run_nodes.sh`

The contents are:

1. `chainrunner` - a Python package for running the chain locally on a single machine. Please check the docstrings inside the python source for API documentation.

2. `run_nodes.py` - an example script showing how to use the `chainrunner` package. It mimics the behavior of `run_nodes.sh`

3`test_catch_up.py` - a script simulating nodes disconnecting from the chain for couple sessions and then reconnecting.

4`test_multiple_restarts.py` - a script simulating repeated crash and recovery of a single validator within one session.
