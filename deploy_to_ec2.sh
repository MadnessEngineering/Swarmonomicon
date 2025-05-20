#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Deploying Swarmonomicon binaries to EC2...${NC}"

# Check if bin directory exists
if [ ! -d "bin" ]; then
    echo -e "${RED}Bin directory not found!${NC}"
    echo -e "${YELLOW}Please run build_from_windows.sh first to build the project.${NC}"
    exit 1
fi

# Test EC2 connection
echo -e "${YELLOW}Testing SSH connection to EC2 instance (eaws)...${NC}"
ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}SSH connection to EC2 instance failed!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. Your SSH configuration includes an 'eaws' host in ~/.ssh/config"
    echo -e "2. The SSH key has the correct permissions"
    echo -e "3. The EC2 instance is running and accessible"
    echo -e "\nExample SSH config entry:"
    echo -e "Host eaws"
    echo -e "    HostName your-ec2-instance-ip-or-dns"
    echo -e "    User ubuntu"
    echo -e "    IdentityFile ~/.ssh/your-ec2-key.pem"
    exit 1
fi

echo -e "${GREEN}SSH connection to EC2 instance successful!${NC}"

# Check if the swarmonomicon directory exists on the remote server
echo -e "${YELLOW}Checking for Swarmonomicon directory on EC2...${NC}"
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

# Make binaries executable
echo -e "${YELLOW}Making binaries executable on EC2...${NC}"
ssh eaws "chmod +x ~/Swarmonomicon/bin/*"

# Ask if user wants to restart services
echo -e "${YELLOW}Would you like to restart services on EC2? (y/n)${NC}"
read -r RESTART

if [[ "$RESTART" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Restarting services on EC2...${NC}"
    
    # Check if restart_services.sh exists
    ssh eaws "if [ -f ~/Swarmonomicon/bin/restart_services.sh ]; then echo 'exists'; else echo 'not_found'; fi" > /tmp/script_check.txt
    
    if grep -q "exists" /tmp/script_check.txt; then
        ssh eaws "cd ~/Swarmonomicon/bin && ./restart_services.sh"
    else
        echo -e "${RED}restart_services.sh not found on EC2!${NC}"
        echo -e "${YELLOW}Please upload restart_services.sh to ~/Swarmonomicon/bin/ on your EC2 instance.${NC}"
    fi
    
    # Clean up temporary file
    if [ -f /tmp/script_check.txt ]; then
        rm /tmp/script_check.txt
    fi
fi

echo -e "${GREEN}Deployment completed!${NC}"
echo -e "${YELLOW}Binaries are now available on your EC2 instance at ~/Swarmonomicon/bin/${NC}" 
