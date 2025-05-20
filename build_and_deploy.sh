#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Preparing to build Swarmonomicon on EC2 instance...${NC}"

# Test SSH connection
./test_ssh.sh

if [ $? -ne 0 ]; then
    echo -e "${RED}SSH connection failed. Please fix SSH configuration and try again.${NC}"
    exit 1
fi

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

# Copy the archive to the EC2 instance
echo -e "${YELLOW}Copying source code to EC2 instance...${NC}"
scp "${TEMP_DIR}/swarmonomicon.tar.gz" eaws:~/Swarmonomicon/
if [ $? -ne 0 ]; then
    echo -e "${RED}Failed to copy source code to EC2.${NC}"
    rm -rf "${TEMP_DIR}"
    exit 1
fi

# Clean up the temporary directory
rm -rf "${TEMP_DIR}"

# SSH to the EC2 instance and build the project
echo -e "${YELLOW}Building project on EC2 instance...${NC}"
ssh eaws << 'EOF'
    cd ~/Swarmonomicon
    echo "Extracting source code..."
    mkdir -p src_temp
    tar -xzf swarmonomicon.tar.gz -C src_temp
    
    # Check if Rust is installed
    if ! command -v rustc &> /dev/null; then
        echo "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # Build the project
    cd src_temp
    echo "Building project..."
    cargo build --release
    
    # Create bin directory if it doesn't exist
    mkdir -p ../bin
    
    # Copy the binaries to bin directory
    echo "Copying binaries..."
    find target/release -maxdepth 1 -type f -executable \
        -not -name "*.d" -not -name "*.rlib" -not -name "*.so" -not -name "*.dylib" \
        -not -name "*.dll" -not -name "*.a" -not -name "build" \
        -exec cp {} ../bin/ \;
    
    # Copy restart_services.sh to the bin directory
    if [ -f restart_services.sh ]; then
        cp restart_services.sh ../bin/
        chmod +x ../bin/restart_services.sh
    fi
    
    # Go back to the main directory
    cd ..
    
    # Clean up the source code
    rm -rf src_temp
    echo "Build completed successfully!"
EOF

if [ $? -ne 0 ]; then
    echo -e "${RED}Build on EC2 failed.${NC}"
    exit 1
fi

echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "${YELLOW}Binaries are now available on your EC2 instance at ~/Swarmonomicon/bin/${NC}"

# Optionally run a command on EC2 to set up or restart services
echo -e "${YELLOW}Would you like to restart services on EC2? (y/n)${NC}"
read -r RESTART

if [[ "$RESTART" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Restarting services on EC2...${NC}"
    ssh eaws "cd ~/Swarmonomicon/bin && chmod +x * && ./restart_services.sh"
fi

echo -e "${GREEN}All done!${NC}"
