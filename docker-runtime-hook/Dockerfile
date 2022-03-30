FROM ubuntu:focal-20211006

RUN apt update && \
 apt install curl -y && \
 apt install unzip -y && \
 apt install git -y && \
 apt install jq -y

WORKDIR aleph-runtime

COPY bin/cliain/target/release/cliain /aleph-runtime/cliain
RUN chmod +x /aleph-runtime/cliain

COPY docker-runtime-hook/entrypoint.sh /aleph-runtime/entrypoint.sh
RUN chmod +x /aleph-runtime/entrypoint.sh

ENTRYPOINT ["./entrypoint.sh"]