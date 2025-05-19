#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Swarmonomicon for EC2 deployment using Docker...${NC}"

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Docker is not installed. Please install Docker and try again.${NC}"
    exit 1
fi

# Create bin directory for binaries if it doesn't exist
mkdir -p bin

# Build using Docker
echo -e "${GREEN}Building Docker image for compilation...${NC}"
docker build -t swarmonomicon-builder -f Dockerfile.build .

if [ $? -ne 0 ]; then
    echo -e "${RED}Error building Docker image! Aborting.${NC}"
    exit 1
fi

# Extract binaries from the Docker container
echo -e "${GREEN}Extracting binaries from Docker container...${NC}"
docker create --name temp-swarmonomicon swarmonomicon-builder
docker cp temp-swarmonomicon:/output/. bin/
docker rm temp-swarmonomicon

if [ $? -ne 0 ]; then
    echo -e "${RED}Error extracting binaries from Docker container! Aborting.${NC}"
    exit 1
fi

# Make binaries executable
chmod +x bin/*

echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "${YELLOW}Preparing for deployment to EC2 instance...${NC}"

# Copy restart_services.sh to bin directory
cp restart_services.sh bin/

# Make sure the EC2 swarmonomicon directory exists
ssh eaws "mkdir -p ~/swarmonomicon"

# Transfer binaries to EC2
echo -e "${YELLOW}Copying binaries to EC2 instance...${NC}"
scp -r bin/* eaws:~/swarmonomicon/

if [ $? -ne 0 ]; then
    echo -e "${RED}Error transferring files to EC2! Make sure SSH is configured correctly.${NC}"
    exit 1
fi

echo -e "${GREEN}Deployment completed successfully!${NC}"
echo -e "${YELLOW}Binaries are now available on your EC2 instance at ~/swarmonomicon/${NC}"

# Optionally run a command on EC2 to set up or restart services
echo -e "${YELLOW}Would you like to restart services on EC2? (y/n)${NC}"
read -r RESTART

if [[ "$RESTART" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Restarting services on EC2...${NC}"
    ssh eaws "cd ~/swarmonomicon && chmod +x * && ./restart_services.sh"
fi

echo -e "${GREEN}Done!${NC}"
echo -e "${YELLOW}Cleaning up...${NC}"
docker rmi swarmonomicon-builder

echo -e "${GREEN}All done!${NC}"
