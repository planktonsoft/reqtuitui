# Use the official Rust image
FROM rust:latest

# Install common development tools, including cargo-watch for hot-reloading
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-watch

# Set the working directory
WORKDIR /usr/src/app

# Note: We do not copy the source code into the image here.
# Instead, we will mount it via docker-compose so that changes on the host
# sync immediately with the container, and target/ cache is preserved.

# Start cargo-watch to watch for changes and restart the application automatically
CMD ["cargo", "watch", "-x", "run"]
