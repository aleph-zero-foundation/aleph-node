FROM ubuntu:focal

WORKDIR client

RUN apt-get update && apt-get install -y libssl-dev ca-certificates

COPY target/release/aleph-e2e-client /usr/local/bin
RUN chmod +x /usr/local/bin/aleph-e2e-client

COPY docker_entrypoint.sh /client/docker_entrypoint.sh
RUN chmod +x /client/docker_entrypoint.sh

ENTRYPOINT ["./docker_entrypoint.sh"]
