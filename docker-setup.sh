#!/bin/bash
# Setup script for Swarmonomicon Docker environment on macOS/Linux

set -e

# Terminal colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}  Swarmonomicon Docker Environment Setup      ${NC}"
echo -e "${BLUE}===============================================${NC}"
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Docker is not installed. Please install Docker Desktop first.${NC}"
    echo "Visit: https://www.docker.com/products/docker-desktop"
    exit 1
fi

# Check Docker version
DOCKER_VERSION=$(docker --version | awk '{print $3}' | tr -d ',')
echo -e "${GREEN}Using Docker version: ${DOCKER_VERSION}${NC}"

# Check if docker-compose is available (either standalone or as plugin)
if command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
    COMPOSE_VERSION=$(docker-compose --version | awk '{print $3}' | tr -d ',')
    echo -e "${GREEN}Using docker-compose version: ${COMPOSE_VERSION}${NC}"
elif docker compose version &> /dev/null; then
    COMPOSE_CMD="docker compose"
    COMPOSE_VERSION=$(docker compose version --short)
    echo -e "${GREEN}Using docker compose plugin version: ${COMPOSE_VERSION}${NC}"
else
    echo -e "${RED}Neither docker-compose nor the docker compose plugin is installed.${NC}"
    echo "Please install Docker Compose and try again."
    exit 1
fi

# Create missing files and directories if needed
if [ ! -d "./config" ]; then
    echo -e "${YELLOW}Creating config directory...${NC}"
    mkdir -p ./config
fi

# Check if mosquitto.conf exists, create if not
if [ ! -f "./config/mosquitto.conf" ]; then
    echo -e "${YELLOW}Creating mosquitto.conf...${NC}"
    cat > ./config/mosquitto.conf << EOF
# Mosquitto MQTT Configuration for Swarmonomicon

# Basic configuration
persistence true
persistence_location /mosquitto/data/
log_dest file /mosquitto/log/mosquitto.log
log_dest stdout

# Default listener
listener 1883
protocol mqtt

# WebSockets listener for web clients
listener 9001
protocol websockets

# Allow anonymous connections with no authentication
# IMPORTANT: This is for development only
# For production, use password_file or another authentication method
allow_anonymous true
EOF
    echo -e "${GREEN}Created mosquitto.conf successfully.${NC}"
fi

# Create convenience scripts
echo -e "${YELLOW}Creating convenience scripts...${NC}"

# Create start-docker.sh if it doesn't exist
if [ ! -f "./start-docker.sh" ]; then
    cat > ./start-docker.sh << EOF
#!/bin/bash

echo "Starting Swarmonomicon Docker environment..."

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
  echo "Error: Docker is not running. Please start Docker and try again."
  exit 1
fi

# Check if docker-compose exists
if command -v docker-compose &> /dev/null; then
  compose_cmd="docker-compose"
elif docker compose version &> /dev/null; then
  compose_cmd="docker compose"
else
  echo "Error: Neither docker-compose nor docker compose plugin found."
  echo "Please install Docker Compose and try again."
  exit 1
fi

# Start the Docker environment
echo "Starting services (MongoDB, MQTT, Ollama, and Swarmonomicon)..."
\$compose_cmd up -d

echo "Waiting for services to initialize..."
sleep 5

echo "⚡ Swarmonomicon Docker environment is running! ⚡"
echo ""
echo "📊 MongoDB is available at localhost:27017"
echo "📡 MQTT broker is available at localhost:1883"
echo "🧠 Ollama AI is available at localhost:11434"
echo "🔮 Swarmonomicon API is available at localhost:3000"
echo ""
echo "To view logs: ./view-logs.sh"
echo "To stop: ./stop-docker.sh"
EOF
    chmod +x ./start-docker.sh
    echo -e "${GREEN}Created start-docker.sh successfully.${NC}"
fi

# Create stop-docker.sh if it doesn't exist
if [ ! -f "./stop-docker.sh" ]; then
    cat > ./stop-docker.sh << EOF
#!/bin/bash

echo "Stopping Docker environment..."

# Check if docker-compose exists
if command -v docker-compose &> /dev/null; then
  compose_cmd="docker-compose"
elif docker compose version &> /dev/null; then
  compose_cmd="docker compose"
else
  echo "Error: Neither docker-compose nor docker compose plugin found."
  echo "Please install Docker Compose and try again."
  exit 1
fi

# Stop all containers
\$compose_cmd down

echo "Docker environment stopped."
EOF
    chmod +x ./stop-docker.sh
    echo -e "${GREEN}Created stop-docker.sh successfully.${NC}"
fi

# Create view-logs.sh if it doesn't exist
if [ ! -f "./view-logs.sh" ]; then
    cat > ./view-logs.sh << EOF
#!/bin/bash

# Check if docker-compose exists
if command -v docker-compose &> /dev/null; then
  compose_cmd="docker-compose"
elif docker compose version &> /dev/null; then
  compose_cmd="docker compose"
else
  echo "Error: Neither docker-compose nor docker compose plugin found."
  echo "Please install Docker Compose and try again."
  exit 1
fi

if [ "\$1" == "" ]; then
  echo "Viewing logs for all services..."
  \$compose_cmd logs -f
else
  echo "Viewing logs for \$1..."
  \$compose_cmd logs -f "\$1"
fi
EOF
    chmod +x ./view-logs.sh
    echo -e "${GREEN}Created view-logs.sh successfully.${NC}"
fi

# Detect system architecture
ARCH=$(uname -m)
if [[ "$ARCH" == "arm64" ]]; then
    echo -e "${YELLOW}Detected Apple Silicon Mac (${ARCH})${NC}"
    echo -e "${YELLOW}Configuring for Apple Silicon...${NC}"
    
    # Update docker-compose.yml for Apple Silicon if needed
    if grep -q "driver: nvidia" "./docker-compose.yml"; then
        echo -e "${YELLOW}Updating docker-compose.yml for Apple Silicon...${NC}"
        sed -i '' 's/driver: nvidia/driver: none/g' ./docker-compose.yml
        echo -e "${GREEN}Updated docker-compose.yml for Apple Silicon.${NC}"
    fi
else
    echo -e "${YELLOW}Detected Intel Mac (${ARCH})${NC}"
fi

echo -e "${GREEN}Setup completed successfully!${NC}"
echo ""
echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}  Next steps:                                 ${NC}"
echo -e "${BLUE}===============================================${NC}"
echo -e "1. Start the environment:  ${YELLOW}./start-docker.sh${NC}"
echo -e "2. View logs:              ${YELLOW}./view-logs.sh${NC}"
echo -e "3. Stop the environment:   ${YELLOW}./stop-docker.sh${NC}"
echo ""
echo -e "For more information, see ${YELLOW}DOCKER.md${NC}"
echo ""

# Ask if user wants to start the environment now
read -p "Would you like to start the Docker environment now? (y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Starting Docker environment..."
    ./start-docker.sh
fi
