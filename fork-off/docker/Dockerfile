FROM ubuntu:jammy-20220531

RUN apt update && \
        apt install ca-certificates -y && \
        apt clean && \
        rm -rf /var/lib/apt/lists/*

WORKDIR app

COPY fork-off /usr/local/bin

ENTRYPOINT ["fork-off"]
