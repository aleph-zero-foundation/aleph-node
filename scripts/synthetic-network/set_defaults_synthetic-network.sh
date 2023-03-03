#!/bin/env bash

set -euo pipefail

source ./scripts/common.sh

function usage() {
    cat << EOF
Usage:
  $0
  This scripts sets network settings for the synthetic-network to some sane defaults.

  --node Node0
    name  of the docker container inside which this script should be executed, default is 'Node0'
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --node)
            NODE="$2"
            shift;shift
            ;;
        --help)
            usage
            shift
            ;;
        *)
            error "Unrecognized argument $1!"
            ;;
    esac
done

NODE=${NODE:-Node0}

docker exec $NODE curl -H "Content-Type: application/json" \
-d \
'{
  "default_link": {
    "ingress": {
      "rate": 27800000,
      "loss": 0,
      "latency": 0,
      "jitter": 0,
      "jitter_strength": 0,
      "reorder_packets": false
    },
    "egress": {
      "rate": 1000000,
      "loss": 0,
      "latency": 0,
      "jitter": 0,
      "jitter_strength": 0,
      "reorder_packets": false
    }
  },
  "flows": [
    {
      "label": "http",
      "flow": {
        "ip": 0,
        "protocol": 6,
        "port_min": 80,
        "port_max": 80
      },
      "link": {
        "ingress": {
          "rate": 96500000,
          "loss": 0,
          "latency": 0,
          "jitter": 0,
          "jitter_strength": 0,
          "reorder_packets": false
        },
        "egress": {
          "rate": 96500000,
          "loss": 0,
          "latency": 0,
          "jitter": 0,
          "jitter_strength": 0,
          "reorder_packets": false
        }
      }
    }
  ]
}' http://localhost:80/qos

exit 0
