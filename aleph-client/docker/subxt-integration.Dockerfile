FROM rustlang/rust:nightly-slim

WORKDIR subxt

RUN cargo install subxt-cli
RUN rustup component add rustfmt --toolchain nightly

COPY docker/subxt-integration-entrypoint.sh /subxt/subxt-integration-entrypoint.sh

RUN chmod +x /subxt/subxt-integration-entrypoint.sh
RUN rustc --version

ENTRYPOINT ["./subxt-integration-entrypoint.sh"]
