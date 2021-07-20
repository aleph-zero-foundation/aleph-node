FROM debian:stretch-slim

WORKDIR node

COPY target/release/aleph-node /node/aleph-node
COPY docker/run_node.sh /node/run_node.sh
RUN mkdir /node/tmp/

ENTRYPOINT ["./run_node.sh"]
