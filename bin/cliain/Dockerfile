FROM ubuntu:jammy-20220531

COPY target/release/cliain /usr/local/bin
RUN chmod +x /usr/local/bin/cliain

ENTRYPOINT ["/usr/local/bin/cliain"]
