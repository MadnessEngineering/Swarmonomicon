# Swarmonomicon Dockerfile
# Multi-platform build for macOS and Windows

# Build stage
FROM rust:1.75-slim-bullseye as builder

# Install system dependencies for both platforms
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    cmake \
    g++ \
    curl \
    libopencv-dev \
    libfreetype6-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
RUN cargo init

# Copy over manifests first (for better layer caching)
COPY Cargo.toml Cargo.lock ./

# Cache dependencies by building an empty project with the manifests
RUN mkdir src && \
    echo "fn main() {println!(\"Placeholder\")}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the full source code
COPY . .

# Build the project with default features
RUN cargo build --release

# Also build with RL features for those who need them
RUN cargo build --release --features rl

# Runtime stage
FROM debian:bullseye-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    libopencv-dev \
    libfreetype6 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binaries from the builder stage
COPY --from=builder /app/target/release/swarm /app/
COPY --from=builder /app/target/release/todo_worker /app/
COPY --from=builder /app/target/release/mcp_todo_server /app/

# Create a directory for models and data
RUN mkdir -p /app/models /app/data

# Set environment variables
ENV RUST_LOG=info

# Expose common ports
EXPOSE 8080 1883

# Default command (can be overridden)
CMD ["/app/swarm"]

# Additional Dockerfiles for specific platforms
# These use the main Dockerfile as a base but add platform-specific optimizations

# Create a macOS-specific Docker configuration
FROM runtime as macos
# macOS-specific optimizations would go here
# Note: Docker for Mac runs Linux containers, so this is more for organization

# Create a Windows-specific Docker configuration
FROM runtime as windows
# Windows-specific configurations would go here
# Note: For true Windows containers, you would use a different base image

# Usage instructions added as Docker labels
LABEL maintainer="Danedens31@gmail.com"
LABEL description="Swarmonomicon - Agent Swarm and Eventbase"
LABEL version="0.1.3"
LABEL usage.common="docker run -p 8080:8080 -p 1883:1883 swarmonomicon"
LABEL usage.rl="docker run -p 8080:8080 -p 1883:1883 -v $(pwd)/models:/app/models swarmonomicon /app/train_flappy" 
