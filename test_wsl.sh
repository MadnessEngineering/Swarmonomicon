#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing WSL build environment...${NC}"

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

# Test WSL availability
echo -e "${YELLOW}Testing WSL availability on Windows machine...${NC}"
ssh salad "wsl --status" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}WSL is not properly set up on the Windows machine!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. WSL is installed on your Windows machine"
    echo -e "2. At least one Linux distribution is installed (preferably Ubuntu)"
    exit 1
fi

echo -e "${GREEN}WSL is available on Windows machine!${NC}"

# Check if Ubuntu distribution is installed
echo -e "${YELLOW}Checking for Ubuntu distribution in WSL...${NC}"
ssh salad "wsl -l -v | grep -i ubuntu" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}Ubuntu distribution not found in WSL!${NC}"
    echo -e "${YELLOW}Please install Ubuntu in WSL with:${NC}"
    echo -e "wsl --install -d Ubuntu"
    exit 1
fi

echo -e "${GREEN}Ubuntu distribution found in WSL!${NC}"

# Test if Rust is installed in WSL
echo -e "${YELLOW}Checking Rust installation in WSL...${NC}"
ssh salad "wsl -d Ubuntu -e rustc --version" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${YELLOW}Rust is not installed in WSL.${NC}"
    echo -e "${YELLOW}Rust will be installed automatically during build process.${NC}"
else
    echo -e "${GREEN}Rust is installed in WSL!${NC}"
fi

# Test access to C: drive from WSL
echo -e "${YELLOW}Testing WSL access to Windows filesystems...${NC}"
ssh salad "wsl -d Ubuntu -e ls /mnt/c" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${RED}Cannot access Windows filesystem from WSL!${NC}"
    echo -e "${YELLOW}Please make sure:${NC}"
    echo -e "1. Windows filesystem is mounted in WSL"
    echo -e "2. You have permission to access it"
    exit 1
fi

echo -e "${GREEN}WSL has access to Windows filesystem!${NC}"

# Check if project directory exists on Windows
echo -e "${YELLOW}Checking for project directory on Windows...${NC}"
ssh salad "if [ -d \"C:\\Users\\Dan\\lab\\madness_interactive\\projects\\common\\Swarmonomicon\" ]; then echo \"exists\"; else echo \"not_found\"; fi" > /tmp/dir_check.txt

if grep -q "not_found" /tmp/dir_check.txt; then
    echo -e "${RED}Project directory not found on Windows!${NC}"
    echo -e "${YELLOW}Expected path: C:\\Users\\Dan\\lab\\madness_interactive\\projects\\common\\Swarmonomicon${NC}"
    rm /tmp/dir_check.txt
    exit 1
fi

rm /tmp/dir_check.txt
echo -e "${GREEN}Project directory found on Windows!${NC}"

# Test EC2 connection if available
echo -e "${YELLOW}Testing SSH connection to EC2 instance (eaws)...${NC}"
ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${YELLOW}SSH connection to EC2 instance not available from Mac.${NC}"
    echo -e "${YELLOW}Will need to transfer files indirectly.${NC}"
else
    echo -e "${GREEN}SSH connection to EC2 instance successful from Mac!${NC}"
    
    # Check if the swarmonomicon directory exists on the remote server
    echo -e "${YELLOW}Checking for Swarmonomicon directory on EC2...${NC}"
    
    ssh eaws "if [ -d ~/Swarmonomicon ]; then echo 'exists'; else echo 'not_found'; fi" > /tmp/dir_check.txt
    
    if grep -q "exists" /tmp/dir_check.txt; then
        echo -e "${GREEN}Found Swarmonomicon directory on EC2.${NC}"
    else
        echo -e "${YELLOW}Creating Swarmonomicon directory on EC2...${NC}"
        ssh eaws "mkdir -p ~/Swarmonomicon"
        
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}Directory created successfully.${NC}"
        else
            echo -e "${RED}Failed to create directory on EC2.${NC}"
            rm /tmp/dir_check.txt
            exit 1
        fi
    fi
    
    rm /tmp/dir_check.txt
fi

# Summary
echo -e "\n${GREEN}=== BUILD ENVIRONMENT SUMMARY ===${NC}"
echo -e "${GREEN}✓${NC} SSH connection to Windows machine (salad) is working"
echo -e "${GREEN}✓${NC} WSL is properly set up on Windows machine"
echo -e "${GREEN}✓${NC} Ubuntu distribution is available in WSL"
echo -e "${GREEN}✓${NC} WSL can access Windows filesystem"
echo -e "${GREEN}✓${NC} Project directory found on Windows"

# Print status of optional requirements
ssh -o BatchMode=yes -o ConnectTimeout=5 eaws echo "Connection successful" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} SSH connection to EC2 instance (eaws) is working"
else
    echo -e "${YELLOW}⚠${NC} SSH connection to EC2 instance (eaws) is not available from Mac"
fi

ssh salad "wsl -d Ubuntu -e rustc --version" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Rust is installed in WSL"
else
    echo -e "${YELLOW}⚠${NC} Rust is not installed in WSL (will be installed during build)"
fi

echo -e "\n${GREEN}The WSL environment is properly set up for cross-compilation!${NC}"
echo -e "${YELLOW}You can now run build_and_deploy_wsl.sh to build and deploy the project.${NC}" 
