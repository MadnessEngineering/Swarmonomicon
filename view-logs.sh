#!/bin/bash

# Terminal colors
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

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

# Check the options
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
  echo -e "${BLUE}Usage:${NC}"
  echo -e "  ${GREEN}./view-logs.sh${NC}               ${YELLOW}# View logs for all services${NC}"
  echo -e "  ${GREEN}./view-logs.sh${NC} ${BLUE}<service>${NC}    ${YELLOW}# View logs for specific service${NC}"
  echo -e "  ${GREEN}./view-logs.sh${NC} ${BLUE}--list${NC}       ${YELLOW}# List available services${NC}"
  echo -e "\n${BLUE}Available services:${NC}"
  $compose_cmd ps --services
  exit 0
elif [ "$1" == "--list" ] || [ "$1" == "-l" ]; then
  echo -e "${BLUE}Available services:${NC}"
  $compose_cmd ps --services
  exit 0
elif [ "$1" == "" ]; then
  echo -e "${YELLOW}Viewing logs for all services...${NC}"
  echo -e "${GREEN}Press Ctrl+C to exit${NC}"
  $compose_cmd logs -f
else
  # Check if the service exists
  if $compose_cmd ps --services | grep -q "^$1$"; then
    echo -e "${YELLOW}Viewing logs for ${GREEN}$1${YELLOW}...${NC}"
    echo -e "${GREEN}Press Ctrl+C to exit${NC}"
    $compose_cmd logs -f "$1"
  else
    echo -e "${RED}Error: Service '$1' not found.${NC}"
    echo -e "${YELLOW}Available services:${NC}"
    $compose_cmd ps --services
    exit 1
  fi
fi 
