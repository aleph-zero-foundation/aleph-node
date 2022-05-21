#!/bin/bash

set -e

DIR=$(dirname "$0")

"$DIR"/run_checks_on_aleph_node.sh
"$DIR"/run_checks_on_excluded_packages.sh
"$DIR"/run_local_e2e_pipeline.sh
