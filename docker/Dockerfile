FROM ubuntu:focal-20210827

EXPOSE 30333 9933 9944

WORKDIR node

COPY target/release/aleph-node /usr/local/bin
RUN chmod +x /usr/local/bin/aleph-node

COPY docker/docker_entrypoint.sh /node/docker_entrypoint.sh
RUN chmod +x /node/docker_entrypoint.sh

ENTRYPOINT ["./docker_entrypoint.sh"]
