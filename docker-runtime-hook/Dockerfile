FROM ubuntu:focal-20211006

RUN apt update && \
 apt install curl -y && \
 apt install unzip -y && \
 apt install git -y && \
 apt install jq -y

WORKDIR aleph-runtime

COPY local-tests/send-runtime/target/release/send_runtime /aleph-runtime/send_runtime
RUN chmod +x /aleph-runtime/send_runtime

COPY docker-runtime-hook/entrypoint.sh /aleph-runtime/entrypoint.sh
RUN chmod +x /aleph-runtime/entrypoint.sh

ENTRYPOINT ["./entrypoint.sh"]