FROM ubuntu:jammy

RUN apt update && \
        apt install curl -y

COPY target/release/chain-bootstrapper /usr/local/bin
RUN chmod +x /usr/local/bin/chain-bootstrapper

ENTRYPOINT ["chain-bootstrapper"]
