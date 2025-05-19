#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing SSH connection to EC2 instance (eaws)...${NC}"

# Test SSH connection
ssh -o BatchMode=yes -o ConnectTimeout=10 eaws echo "Connection successful" > /dev/null 2>&1

if [ $? -eq 0 ]; then
    echo -e "${GREEN}SSH connection successful!${NC}"
    
    # Check if the swarmonomicon directory exists on the remote server
    echo -e "${YELLOW}Checking for swarmonomicon directory on EC2...${NC}"
    
    ssh eaws "if [ -d ~/swarmonomicon ]; then echo 'exists'; else echo 'not_found'; fi" > /tmp/dir_check.txt
    
    if grep -q "exists" /tmp/dir_check.txt; then
        echo -e "${GREEN}Found swarmonomicon directory on EC2.${NC}"
    else
        echo -e "${YELLOW}Creating swarmonomicon directory on EC2...${NC}"
        ssh eaws "mkdir -p ~/swarmonomicon"
        
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}Directory created successfully.${NC}"
        else
            echo -e "${RED}Failed to create directory on EC2.${NC}"
            exit 1
        fi
    fi
    
    # Clean up temporary file
    rm /tmp/dir_check.txt
    
    echo -e "${GREEN}All checks passed! Your EC2 instance is ready for deployment.${NC}"
else
    echo -e "${RED}SSH connection failed!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. Your SSH configuration includes an 'eaws' host in ~/.ssh/config"
    echo -e "2. The SSH key has the correct permissions (chmod 600 ~/.ssh/your-key.pem)"
    echo -e "3. The EC2 instance is running and accessible"
    echo -e "\nExample SSH config entry:"
    echo -e "Host eaws"
    echo -e "    HostName your-ec2-instance-ip-or-dns"
    echo -e "    User ubuntu"
    echo -e "    IdentityFile ~/.ssh/your-ec2-key.pem"
    exit 1
fi 
