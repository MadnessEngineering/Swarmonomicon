#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Swarmonomicon in WSL and deploying to EC2...${NC}"

# Test WSL availability
echo -e "${YELLOW}Testing WSL availability...${NC}"
wsl --status > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}WSL is not properly set up!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. WSL is installed on your Windows machine"
    echo -e "2. At least one Linux distribution is installed (preferably Ubuntu)"
    exit 1
fi

echo -e "${GREEN}WSL is available!${NC}"

# Check if Ubuntu distribution is installed
echo -e "${YELLOW}Checking for Ubuntu distribution in WSL...${NC}"
wsl -l -v | grep -i ubuntu > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}Ubuntu distribution not found in WSL!${NC}"
    echo -e "${YELLOW}Please install Ubuntu in WSL with:${NC}"
    echo -e "wsl --install -d Ubuntu"
    exit 1
fi

echo -e "${GREEN}Ubuntu distribution found in WSL!${NC}"

# Enter WSL and build
echo -e "${YELLOW}Building project in WSL...${NC}"
wsl -d Ubuntu << 'WSLEOF'
    cd $(wslpath "$(pwd)")
    
    # Check if Rust is installed
    if ! command -v rustc &> /dev/null; then
        echo "Installing Rust in WSL..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # Make sure we have latest stable Rust
    rustup update stable
    
    # Install dependencies if needed
    sudo apt update
    sudo apt install -y pkg-config libssl-dev libcurl4-openssl-dev cmake gcc g++ libc6-dev git
    
    # Build the project for Linux
    echo "Building project for Linux..."
    cargo build --release
    
    # Create bin directory
    mkdir -p bin
    
    # Copy the binaries to bin directory
    echo "Copying binaries..."
    find target/release -maxdepth 1 -type f -executable \
        -not -name "*.d" -not -name "*.rlib" -not -name "*.so" -not -name "*.dylib" \
        -not -name "*.dll" -not -name "*.a" -not -name "build" \
        -exec cp {} bin/ \;
    
    # Copy restart_services.sh to the bin directory
    if [ -f restart_services.sh ]; then
        cp restart_services.sh bin/
        chmod +x bin/restart_services.sh
    fi
    
    echo "Build completed successfully!"
WSLEOF

if [ $? -ne 0 ]; then
    echo -e "${RED}Build in WSL failed.${NC}"
    exit 1
fi

echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "${YELLOW}Binaries are available in the bin directory${NC}"

# Test EC2 connection
echo -e "${YELLOW}Testing SSH connection to EC2 instance (eaws)...${NC}"
ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${YELLOW}SSH connection to EC2 instance not available.${NC}"
    echo -e "${YELLOW}Please set up SSH to your EC2 instance to enable deployment.${NC}"
    echo -e "${YELLOW}You can manually copy the binaries from the bin directory to your EC2 instance.${NC}"
    exit 0
fi

echo -e "${GREEN}SSH connection to EC2 instance successful!${NC}"

# Ask if user wants to deploy to EC2
echo -e "${YELLOW}Would you like to deploy to EC2? (y/n)${NC}"
read -r DEPLOY

if [[ "$DEPLOY" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Deploying to EC2...${NC}"
    
    # Check if the swarmonomicon directory exists on the remote server
    ssh eaws "if [ ! -d ~/Swarmonomicon ]; then mkdir -p ~/Swarmonomicon; fi"
    ssh eaws "if [ ! -d ~/Swarmonomicon/bin ]; then mkdir -p ~/Swarmonomicon/bin; fi"
    
    # Copy binaries to EC2
    echo -e "${YELLOW}Copying binaries to EC2...${NC}"
    scp -r bin/* eaws:~/Swarmonomicon/bin/
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Failed to copy binaries to EC2.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Binaries copied to EC2 successfully!${NC}"
    
    # Ask if user wants to restart services
    echo -e "${YELLOW}Would you like to restart services on EC2? (y/n)${NC}"
    read -r RESTART
    
    if [[ "$RESTART" =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Restarting services on EC2...${NC}"
        ssh eaws "cd ~/Swarmonomicon/bin && chmod +x * && ./restart_services.sh"
    fi
    
    echo -e "${GREEN}Deployment completed!${NC}"
fi

echo -e "${GREEN}All done!${NC}" 
