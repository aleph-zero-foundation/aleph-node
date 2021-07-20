FROM debian:stretch-slim

WORKDIR node

COPY target/release/aleph-node /node/aleph-node
RUN mkdir /node/tmp/

ENTRYPOINT ["./aleph-node"]
