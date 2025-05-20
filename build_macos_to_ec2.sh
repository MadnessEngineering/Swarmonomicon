#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Swarmonomicon on macOS and deploying to EC2...${NC}"

# Check if we're running on macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo -e "${RED}This script must be run on macOS.${NC}"
    exit 1
fi

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Docker is not installed. Please install Docker first.${NC}"
    exit 1
fi

# Check if Docker is running
if ! docker info &> /dev/null; then
    echo -e "${RED}Docker daemon is not running. Please start Docker first.${NC}"
    exit 1
fi

echo -e "${YELLOW}Building Docker image for cross-compilation...${NC}"

# Create a temporary Dockerfile
cat > Dockerfile.macos-build << 'EOL'
FROM rust:latest

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libcurl4-openssl-dev cmake gcc g++ libc6-dev git \
    libxcb-shape0-dev libxcb-xfixes0-dev

WORKDIR /app

# Copy the source code
COPY . .

# Build the project
RUN cargo build --release --bin mqtt_intake --target x86_64-unknown-linux-gnu

# Create a directory for the binaries
RUN mkdir -p /output && \
    find target/release -maxdepth 1 -type f -executable \
    -not -name "*.d" -not -name "*.rlib" -not -name "*.so" -not -name "*.dylib" \
    -not -name "*.dll" -not -name "*.a" -not -name "build" \
    -exec cp {} /output/ \;

# Copy the restart_services.sh script if it exists
RUN if [ -f restart_services.sh ]; then cp restart_services.sh /output/ && chmod +x /output/restart_services.sh; fi
EOL

# Build the Docker image
docker build -t swarmonomicon-builder -f Dockerfile.macos-build .

if [ $? -ne 0 ]; then
    echo -e "${RED}Docker build failed. See errors above.${NC}"
    rm Dockerfile.macos-build
    exit 1
fi

echo -e "${GREEN}Docker image built successfully!${NC}"

# Create bin directory
mkdir -p bin

# Extract binaries from the Docker container
echo -e "${YELLOW}Extracting binaries from Docker container...${NC}"
docker create --name swarmonomicon-extract swarmonomicon-builder
docker cp swarmonomicon-extract:/output/. bin/
docker rm swarmonomicon-extract

echo -e "${GREEN}Binaries extracted to bin directory!${NC}"

# Clean up
rm Dockerfile.macos-build

# Ask if user wants to deploy to EC2
echo -e "${YELLOW}Would you like to deploy to EC2? (y/n)${NC}"
read -r DEPLOY

if [[ "$DEPLOY" =~ ^[Yy]$ ]]; then
    # Test EC2 connection
    echo -e "${YELLOW}Testing SSH connection to EC2 instance (eaws)...${NC}"
    ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1

    if [ $? -ne 0 ]; then
        echo -e "${RED}SSH connection to EC2 instance failed!${NC}"
        echo -e "${YELLOW}Please make sure:${NC}"
        echo -e "1. Your SSH configuration includes an 'eaws' host in ~/.ssh/config"
        echo -e "2. The SSH key has the correct permissions"
        echo -e "3. The EC2 instance is running and accessible"
        exit 1
    fi

    echo -e "${GREEN}SSH connection to EC2 instance successful!${NC}"

    # Create directories on EC2
    echo -e "${YELLOW}Setting up directories on EC2...${NC}"
    ssh eaws "mkdir -p ~/Swarmonomicon/bin"

    # Copy binaries to EC2
    echo -e "${YELLOW}Copying binaries to EC2...${NC}"
    scp -r bin/* eaws:~/Swarmonomicon/bin/

    if [ $? -ne 0 ]; then
        echo -e "${RED}Failed to copy binaries to EC2.${NC}"
        exit 1
    fi

    echo -e "${GREEN}Binaries copied to EC2 successfully!${NC}"

    # Make binaries executable
    ssh eaws "chmod +x ~/Swarmonomicon/bin/*"

    # Ask if user wants to restart services
    echo -e "${YELLOW}Would you like to restart services on EC2? (y/n)${NC}"
    read -r RESTART

    if [[ "$RESTART" =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Restarting services on EC2...${NC}"
        ssh eaws "cd ~/Swarmonomicon/bin && ./restart_services.sh"
    fi

    echo -e "${GREEN}Deployment completed!${NC}"
    echo -e "${YELLOW}Binaries are now available on your EC2 instance at ~/Swarmonomicon/bin/${NC}"
fi

echo -e "${GREEN}All done!${NC}"
