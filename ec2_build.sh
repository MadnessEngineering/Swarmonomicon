#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Swarmonomicon and deploying to EC2...${NC}"

# Check if we're running in Ubuntu
if ! grep -q "Ubuntu" /etc/os-release; then
    echo -e "${RED}This script must be run inside Ubuntu Linux.${NC}"
    echo -e "${YELLOW}Please run this script from within your WSL Ubuntu environment.${NC}"
    exit 1
fi

# Ensure dependencies are installed
echo -e "${YELLOW}Installing required dependencies...${NC}"
sudo apt update
sudo apt install -y pkg-config libssl-dev libcurl4-openssl-dev cmake gcc g++ libc6-dev git

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo -e "${GREEN}Rust is already installed.${NC}"
    rustup update stable
fi

# Build the project
echo -e "${YELLOW}Building project...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed. See errors above.${NC}"
    exit 1
fi

echo -e "${GREEN}Build completed successfully!${NC}"

# Create bin directory and copy binaries
echo -e "${YELLOW}Copying binaries to bin directory...${NC}"
mkdir -p bin
find target/release -maxdepth 1 -type f -executable \
    -not -name "*.d" -not -name "*.rlib" -not -name "*.so" -not -name "*.dylib" \
    -not -name "*.dll" -not -name "*.a" -not -name "build" \
    -exec cp {} bin/ \;

# Copy restart_services.sh if it exists
if [ -f restart_services.sh ]; then
    cp restart_services.sh bin/
    chmod +x bin/restart_services.sh
fi

echo -e "${GREEN}Binaries prepared in bin directory.${NC}"

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
