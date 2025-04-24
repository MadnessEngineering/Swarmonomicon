#!/bin/bash

# Terminal colors
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Stopping Docker environment...${NC}"

# Check if docker-compose exists
if command -v docker-compose &> /dev/null; then
  compose_cmd="docker-compose"
elif docker compose version &> /dev/null; then
  compose_cmd="docker compose"
else
  echo -e "${RED}Error: Neither docker-compose nor docker compose plugin found.${NC}"
  echo "Please install Docker Compose and try again."
  exit 1
fi

# Check for clean flag
if [ "$1" == "--clean" ] || [ "$1" == "-c" ]; then
  echo -e "${YELLOW}Stopping and removing all containers, networks, and volumes...${NC}"
  $compose_cmd down -v
  echo -e "${GREEN}Docker environment stopped and volumes removed.${NC}"
  echo -e "${YELLOW}Note: You will need to recreate volumes and re-pull models when you restart.${NC}"
else
  # Stop all containers
  $compose_cmd down
  echo -e "${GREEN}Docker environment stopped.${NC}"
  echo -e "${YELLOW}Tip: Use ${NC}./stop-docker.sh --clean${YELLOW} to remove volumes and start fresh.${NC}"
fi 
