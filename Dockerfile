# Use Ubuntu as the base image
FROM debian:latest

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive
ENV TERM=xterm

# Install required packages
RUN apt-get update && \
    apt-get install -y curl build-essential pkg-config libssl-dev git protobuf-compiler clang libclang-dev llvm-dev librocksdb-dev jq make && \
    rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Verify Rust installation
RUN rustc --version && cargo --version

# Copy the source code into the image
COPY . /app
WORKDIR /app

# Build the Rust project
RUN cargo build --release && \
    rm -rf /app/target/debug

# Expose the necessary ports
EXPOSE 30333 30343 9944

# Make the release and scripts directories and its contents executable
# RUN chmod +x /app/target/release
# RUN chmod +x /app/scripts

RUN chmod +x /app


# Keep the container running
CMD ["/bin/sh", "-c", "/app/scripts/run_nodes.sh"]
# CMD ["tail", "-f", "/dev/null"]
# CMD ["/app/scripts/run_nodes.sh"]

# # Copy and make the entrypoint script executable
# COPY entrypoint.sh /usr/local/bin/entrypoint.sh
# RUN chmod +x /usr/local/bin/entrypoint.sh

# # Command to run when the container starts
# ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
