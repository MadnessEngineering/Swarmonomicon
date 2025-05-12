# Swarmonomicon Dockerfile
# Multi-platform build for macOS and Windows

# Build stage
FROM rust:1.77-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    gcc \
    g++ \
    libc6-dev \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
RUN cargo init

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies only (this will be cached unless dependencies change)
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY . .

# Build the application with optimizations
RUN RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release

# Create runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder
WORKDIR /app
COPY --from=builder /app/target/release/swarm /app/swarm
COPY --from=builder /app/target/release/todo_worker /app/todo_worker
COPY --from=builder /app/target/release/mcp_todo_server /app/mcp_todo_server

# Set environment variables
ENV RUST_LOG=info
ENV AI_ENDPOINT=http://ollama:11434/api/generate
ENV AI_MODEL=qwen2.5-7b-instruct
ENV RTK_MONGO_URI=mongodb://mongodb:27017
ENV RTK_MONGO_DB=swarmonomicon
ENV MQTT_HOST=mosquitto
ENV MQTT_PORT=1883

# Create a non-root user to run the application
RUN useradd -m swarmuser
RUN chown -R swarmuser:swarmuser /app
USER swarmuser

# Add healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Expose the API port
EXPOSE 3000

# Command to run the application
CMD ["./swarm"]

# Additional Dockerfiles for specific platforms
# These use the main Dockerfile as a base but add platform-specific optimizations

# Create a macOS-specific Docker configuration
FROM debian:bookworm-slim AS macos
# macOS-specific optimizations would go here
# Note: Docker for Mac runs Linux containers, so this is more for organization

# Create a Windows-specific Docker configuration
FROM debian:bookworm-slim AS windows
# Windows-specific configurations would go here
# Note: For true Windows containers, you would use a different base image

# Usage instructions added as Docker labels
LABEL maintainer="Danedens31@gmail.com"
LABEL description="Swarmonomicon - Agent Swarm and Eventbase"
LABEL version="0.1.4"
LABEL usage.common="docker run -p 3000:3000 -p 1883:1883 swarmonomicon"
LABEL usage.rl="docker run -p 3000:3000 -p 1883:1883 -v $(pwd)/models:/app/models swarmonomicon /app/train_flappy" 
