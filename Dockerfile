# Use the official Rust image to build the app
FROM rust:latest AS builder

# Set working directory
WORKDIR /usr/src/interchannel

# Copy the manifest and build dependencies first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the release version
RUN cargo build --release

# Use a minimal base image for runtime
FROM debian:bookworm-slim

# Create a non-root user
RUN useradd -m botuser && apt update && apt install -y libssl3 ca-certificates

# Create working dir and copy the compiled binary
WORKDIR /home/botuser
COPY --from=builder /usr/src/interchannel/target/release/interchannel_message_mover .

RUN touch logs.txt
# Use non-root user
USER botuser
# Run the bot
CMD ["./interchannel_message_mover"]