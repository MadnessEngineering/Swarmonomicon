#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Swarmonomicon using WSL and deploying to EC2...${NC}"

# Test SSH connection to Windows machine
echo -e "${YELLOW}Testing SSH connection to Windows machine (salad)...${NC}"
ssh -o BatchMode=yes -o ConnectTimeout=10 salad echo "Connection successful" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}SSH connection to Windows machine failed!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. Your SSH configuration includes a 'salad' host in ~/.ssh/config"
    echo -e "2. The SSH key has the correct permissions"
    echo -e "3. The Windows machine is running and accessible"
    exit 1
fi

echo -e "${GREEN}SSH connection to Windows machine successful!${NC}"

# Create a temporary directory for the source code
echo -e "${YELLOW}Creating temp directory for the source code...${NC}"
TEMP_DIR=$(mktemp -d)
echo -e "${GREEN}Created temp directory: ${TEMP_DIR}${NC}"

# Create archive of the source code
echo -e "${YELLOW}Creating archive of the source code...${NC}"
git archive --format=tar.gz --output="${TEMP_DIR}/swarmonomicon.tar.gz" HEAD
if [ $? -ne 0 ]; then
    echo -e "${RED}Failed to create source code archive.${NC}"
    rm -rf "${TEMP_DIR}"
    exit 1
fi

# Copy the archive to the Windows machine
echo -e "${YELLOW}Copying source code to Windows machine (salad)...${NC}"
scp "${TEMP_DIR}/swarmonomicon.tar.gz" salad:/tmp/
if [ $? -ne 0 ]; then
    echo -e "${RED}Failed to copy source code to Windows.${NC}"
    rm -rf "${TEMP_DIR}"
    exit 1
fi

# Clean up the temporary directory
rm -rf "${TEMP_DIR}"

# SSH to the Windows machine, extract the code in WSL, build it, and transfer to EC2
echo -e "${YELLOW}Building project on WSL and transferring to EC2...${NC}"
ssh salad << 'EOF'
    # Enter WSL
    wsl -d Ubuntu << 'WSLEOF'
        cd /tmp
        echo "Extracting source code..."
        mkdir -p swarmonomicon_build
        tar -xzf swarmonomicon.tar.gz -C swarmonomicon_build
        cd swarmonomicon_build
        
        # Check if Rust is installed in WSL
        if ! command -v rustc &> /dev/null; then
            echo "Installing Rust in WSL..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        fi
        
        # Make sure we have latest stable Rust
        rustup update stable
        
        # Build the project for Linux
        echo "Building project for Linux..."
        cargo build --release
        
        # Create bin directory
        mkdir -p /tmp/swarmonomicon_bin
        
        # Copy the binaries to bin directory
        echo "Copying binaries..."
        find target/release -maxdepth 1 -type f -executable \
            -not -name "*.d" -not -name "*.rlib" -not -name "*.so" -not -name "*.dylib" \
            -not -name "*.dll" -not -name "*.a" -not -name "build" \
            -exec cp {} /tmp/swarmonomicon_bin/ \;
        
        # Copy restart_services.sh to the bin directory
        if [ -f restart_services.sh ]; then
            cp restart_services.sh /tmp/swarmonomicon_bin/
            chmod +x /tmp/swarmonomicon_bin/restart_services.sh
        fi
        
        # Check if ssh to EC2 is set up in WSL
        if ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1; then
            echo "Transferring binaries directly from WSL to EC2..."
            ssh eaws "mkdir -p ~/Swarmonomicon/bin"
            scp /tmp/swarmonomicon_bin/* eaws:~/Swarmonomicon/bin/
            ssh eaws "chmod +x ~/Swarmonomicon/bin/*"
            echo "Transfer from WSL completed!"
        else
            echo "SSH to EC2 not configured in WSL, will transfer files from Mac later..."
            # Create a tarball of the binaries for transfer from Mac
            cd /tmp
            tar -czf swarmonomicon_bin.tar.gz -C swarmonomicon_bin .
            cp swarmonomicon_bin.tar.gz /mnt/c/Users/Dan/Downloads/
            echo "Binaries packaged and copied to Windows Downloads folder"
        fi
        
        # Clean up in WSL
        rm -rf /tmp/swarmonomicon_build
        rm -rf /tmp/swarmonomicon_bin
        rm -f /tmp/swarmonomicon.tar.gz
WSLEOF
EOF

if [ $? -ne 0 ]; then
    echo -e "${RED}Build on WSL failed.${NC}"
    exit 1
fi

# Check if we need to handle transfer from Mac to EC2
if scp salad:/mnt/c/Users/Dan/Downloads/swarmonomicon_bin.tar.gz /tmp/ > /dev/null 2>&1; then
    echo -e "${YELLOW}Transferring binaries from Mac to EC2...${NC}"
    ssh eaws "mkdir -p ~/Swarmonomicon/bin"
    scp /tmp/swarmonomicon_bin.tar.gz eaws:~/Swarmonomicon/
    ssh eaws "cd ~/Swarmonomicon && tar -xzf swarmonomicon_bin.tar.gz -C bin/ && chmod +x bin/* && rm swarmonomicon_bin.tar.gz"
    rm -f /tmp/swarmonomicon_bin.tar.gz
    echo -e "${GREEN}Transfer from Mac completed!${NC}"
fi

echo -e "${GREEN}Build and deployment completed successfully!${NC}"
echo -e "${YELLOW}Binaries are now available on your EC2 instance at ~/Swarmonomicon/bin/${NC}"

# Optionally run a command on EC2 to set up or restart services
echo -e "${YELLOW}Would you like to restart services on EC2? (y/n)${NC}"
read -r RESTART

if [[ "$RESTART" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Restarting services on EC2...${NC}"
    ssh eaws "cd ~/Swarmonomicon/bin && ./restart_services.sh"
fi

echo -e "${GREEN}All done!${NC}" 
