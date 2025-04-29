#!/bin/bash

# Terminal colors
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Starting Swarmonomicon Docker environment...${NC}"

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
  echo -e "${RED}Error: Docker is not running. Please start Docker and try again.${NC}"
  exit 1
fi

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

# Process command line arguments
build=false
pull=false

while [[ "$#" -gt 0 ]]; do
  case $1 in
    --build|-b) build=true; shift ;;
    --pull|-p) pull=true; shift ;;
    --help|-h)
      echo -e "${BLUE}Usage:${NC}"
      echo -e "  ${GREEN}./start-docker.sh${NC}          ${YELLOW}# Start the Docker environment${NC}"
      echo -e "  ${GREEN}./start-docker.sh${NC} ${BLUE}--build${NC}  ${YELLOW}# Rebuild containers before starting${NC}"
      echo -e "  ${GREEN}./start-docker.sh${NC} ${BLUE}--pull${NC}   ${YELLOW}# Pull latest images before starting${NC}"
      exit 0
      ;;
    *) echo -e "${RED}Unknown parameter: $1${NC}"; exit 1 ;;
  esac
done

# Rebuild containers if requested
if [ "$build" = true ]; then
  echo -e "${YELLOW}Rebuilding containers...${NC}"
  $compose_cmd build
fi

# Pull latest images if requested
if [ "$pull" = true ]; then
  echo -e "${YELLOW}Pulling latest images...${NC}"
  $compose_cmd pull
fi

# Start the Docker environment
echo -e "${YELLOW}Starting services (MongoDB, MQTT, Ollama, and Swarmonomicon)...${NC}"
$compose_cmd up -d

echo -e "${YELLOW}Waiting for services to initialize...${NC}"
sleep 5

# Check if all services are running
services_running=true
for service in $($compose_cmd ps --services); do
  status=$($compose_cmd ps --format "table {{.Status}}" $service | tail -n +2 | grep -v "running")
  if [ ! -z "$status" ]; then
    services_running=false
    echo -e "${RED}Warning: Service $service is not running properly.${NC}"
  fi
done

if [ "$services_running" = true ]; then
  echo -e "${GREEN}⚡ Swarmonomicon Docker environment is running! ⚡${NC}"
else
  echo -e "${YELLOW}Some services may not be running properly. Check with ${GREEN}docker compose ps${NC}"
fi

echo ""
echo -e "${BLUE}📊 MongoDB:${NC} localhost:27017"
echo -e "${BLUE}📡 MQTT broker:${NC} localhost:1883"
echo -e "${BLUE}🧠 Ollama AI:${NC} localhost:11434"
echo -e "${BLUE}🔮 Swarmonomicon API:${NC} localhost:3000"
echo ""
echo -e "${YELLOW}Commands:${NC}"
echo -e "  ${GREEN}./view-logs.sh${NC}          ${YELLOW}# View logs from all containers${NC}"
echo -e "  ${GREEN}./view-logs.sh${NC} ${BLUE}<service>${NC} ${YELLOW}# View logs from a specific service${NC}"
echo -e "  ${GREEN}./stop-docker.sh${NC}        ${YELLOW}# Stop all containers${NC}" 
