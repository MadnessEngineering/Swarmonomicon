#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing build environment on Windows with WSL...${NC}"

# Test WSL availability
echo -e "${YELLOW}Testing WSL availability...${NC}"
wsl --status > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}WSL is not properly set up!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. WSL is installed on your Windows machine"
    echo -e "2. At least one Linux distribution is installed (preferably Ubuntu)"
    echo -e "\nTo install WSL, run this command as Administrator in PowerShell:"
    echo -e "wsl --install"
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

# Test if Rust is installed in WSL
echo -e "${YELLOW}Checking Rust installation in WSL...${NC}"
wsl -d Ubuntu -e rustc --version > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${YELLOW}Rust is not installed in WSL.${NC}"
    echo -e "${YELLOW}Rust will be installed automatically during build process.${NC}"
else
    echo -e "${GREEN}Rust is installed in WSL!${NC}"
fi

# Test current project access in WSL
echo -e "${YELLOW}Testing project access in WSL...${NC}"
PROJECT_PATH=$(pwd)
wsl -d Ubuntu -e ls "$(wslpath -u "${PROJECT_PATH}")" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}Cannot access project directory from WSL!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. Your current directory is accessible from WSL"
    echo -e "2. You have permission to access it"
    echo -e "3. WSL integration is enabled in Windows"
    exit 1
fi

echo -e "${GREEN}Project is accessible from WSL!${NC}"

# Test EC2 connection if available
echo -e "${YELLOW}Testing SSH connection to EC2 instance (eaws)...${NC}"
ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${YELLOW}SSH connection to EC2 instance not available.${NC}"
    echo -e "${YELLOW}To deploy to EC2, you need to configure SSH access.${NC}"
    echo -e "${YELLOW}Add the following to your ~/.ssh/config file:${NC}"
    echo -e "Host eaws"
    echo -e "    HostName your-ec2-instance-ip-or-dns"
    echo -e "    User ubuntu"
    echo -e "    IdentityFile ~/.ssh/your-ec2-key.pem"
else
    echo -e "${GREEN}SSH connection to EC2 instance successful!${NC}"
    
    # Check if the swarmonomicon directory exists on the remote server
    echo -e "${YELLOW}Checking for Swarmonomicon directory on EC2...${NC}"
    
    ssh eaws "if [ -d ~/Swarmonomicon ]; then echo 'exists'; else echo 'not_found'; fi" > /tmp/dir_check.txt
    
    if grep -q "exists" /tmp/dir_check.txt; then
        echo -e "${GREEN}Found Swarmonomicon directory on EC2.${NC}"
    else
        echo -e "${YELLOW}Swarmonomicon directory not found on EC2.${NC}"
        echo -e "${YELLOW}It will be created during deployment.${NC}"
    fi
    
    # Clean up temporary file if it exists
    if [ -f /tmp/dir_check.txt ]; then
        rm /tmp/dir_check.txt
    fi
fi

# Check for dependencies in WSL
echo -e "${YELLOW}Checking for required dependencies in WSL...${NC}"
wsl -d Ubuntu bash -c 'dpkg -l | grep -E "pkg-config|libssl-dev|libcurl4-openssl-dev|cmake|gcc|g[+][+]|libc6-dev|git" | wc -l' > /tmp/dep_check.txt

DEP_COUNT=$(cat /tmp/dep_check.txt)
if [ "$DEP_COUNT" -lt 8 ]; then
    echo -e "${YELLOW}Some dependencies may be missing in WSL.${NC}"
    echo -e "${YELLOW}They will be installed during the build process.${NC}"
else
    echo -e "${GREEN}Required dependencies are available in WSL!${NC}"
fi

# Clean up temporary file
if [ -f /tmp/dep_check.txt ]; then
    rm /tmp/dep_check.txt
fi

# Summary
echo -e "\n${GREEN}=== BUILD ENVIRONMENT SUMMARY ===${NC}"
echo -e "${GREEN}✓${NC} WSL is properly set up"
echo -e "${GREEN}✓${NC} Ubuntu distribution is available in WSL"
echo -e "${GREEN}✓${NC} Project is accessible from WSL"

# Print status of optional requirements
wsl -d Ubuntu -e rustc --version > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Rust is installed in WSL"
else
    echo -e "${YELLOW}⚠${NC} Rust is not installed in WSL (will be installed during build)"
fi

ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} SSH connection to EC2 instance (eaws) is working"
else
    echo -e "${YELLOW}⚠${NC} SSH connection to EC2 instance (eaws) is not available"
fi

if [ "$DEP_COUNT" -ge 8 ]; then
    echo -e "${GREEN}✓${NC} Required dependencies are available in WSL"
else
    echo -e "${YELLOW}⚠${NC} Some dependencies may be missing in WSL"
fi

echo -e "\n${GREEN}The Windows WSL environment is prepared for building!${NC}"
echo -e "${YELLOW}You can now run build_from_windows.sh to build and deploy the project.${NC}" 
