FROM ubuntu:jammy-20220531

RUN apt update && \
    apt install wget -y && \
    apt clean

RUN apt update && \
    apt install ca-certificates -y && \
    apt clean && \
    rm -rf /var/lib/apt/lists/*

RUN update-ca-certificates

RUN wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.16_amd64.deb
RUN dpkg -i libssl1.1_1.1.1f-1ubuntu2.16_amd64.deb

COPY target/release/cliain /usr/local/bin
RUN chmod +x /usr/local/bin/cliain

ENTRYPOINT ["/usr/local/bin/cliain"]

