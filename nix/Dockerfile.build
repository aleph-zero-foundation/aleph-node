FROM nixos/nix@sha256:f0c68f870c655d8d96658ca762a0704a30704de22d16b4956e762a2ddfbccb09

COPY nix/nix-build.sh /node/
COPY . /node/build
RUN nix-env -i patchelf && \
    nix-collect-garbage -d && \
    chmod +x /node/nix-build.sh

WORKDIR /node/build
RUN /node/nix-build.sh

ENTRYPOINT ["/node/nix-build.sh"]
CMD []