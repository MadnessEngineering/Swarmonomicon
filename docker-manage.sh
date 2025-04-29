#!/bin/bash

# Terminal colors
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

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

# Display help
show_help() {
  echo -e "${BLUE}Swarmonomicon Docker Management Tool${NC}"
  echo -e ""
  echo -e "${GREEN}Usage:${NC}"
  echo -e "  ${YELLOW}./docker-manage.sh${NC} ${BLUE}[command]${NC}"
  echo -e ""
  echo -e "${GREEN}Commands:${NC}"
  echo -e "  ${BLUE}start${NC}       Start the Docker environment"
  echo -e "  ${BLUE}stop${NC}        Stop the Docker environment"
  echo -e "  ${BLUE}restart${NC}     Restart the Docker environment"
  echo -e "  ${BLUE}logs${NC}        View logs from all services"
  echo -e "  ${BLUE}logs${NC} ${CYAN}<service>${NC}  View logs from a specific service"
  echo -e "  ${BLUE}ps${NC}          List running services"
  echo -e "  ${BLUE}build${NC}       Rebuild all services"
  echo -e "  ${BLUE}pull${NC}        Pull latest images"
  echo -e "  ${BLUE}clean${NC}       Stop and remove all containers, networks, and volumes"
  echo -e "  ${BLUE}status${NC}      Show the status of all services"
  echo -e "  ${BLUE}model${NC}       Pull the AI model"
  echo -e "  ${BLUE}help${NC}        Show this help message"
  echo -e ""
  echo -e "${GREEN}Examples:${NC}"
  echo -e "  ${YELLOW}./docker-manage.sh${NC} ${BLUE}start${NC}     # Start all services"
  echo -e "  ${YELLOW}./docker-manage.sh${NC} ${BLUE}logs${NC} ${CYAN}mongodb${NC} # View MongoDB logs"
  echo -e ""
}

# Start the Docker environment
start_docker() {
  echo -e "${YELLOW}Starting Swarmonomicon Docker environment...${NC}"
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
}

# Stop the Docker environment
stop_docker() {
  echo -e "${YELLOW}Stopping Docker environment...${NC}"
  $compose_cmd down
  echo -e "${GREEN}Docker environment stopped.${NC}"
}

# Clean the Docker environment (remove volumes)
clean_docker() {
  echo -e "${YELLOW}Stopping and removing all containers, networks, and volumes...${NC}"
  $compose_cmd down -v
  echo -e "${GREEN}Docker environment stopped and volumes removed.${NC}"
  echo -e "${YELLOW}Note: You will need to recreate volumes and re-pull models when you restart.${NC}"
}

# Show logs
show_logs() {
  local service=$1
  if [ -z "$service" ]; then
    echo -e "${YELLOW}Viewing logs for all services...${NC}"
    echo -e "${GREEN}Press Ctrl+C to exit${NC}"
    $compose_cmd logs -f
  else
    # Check if the service exists
    if $compose_cmd ps --services | grep -q "^$service$"; then
      echo -e "${YELLOW}Viewing logs for ${GREEN}$service${YELLOW}...${NC}"
      echo -e "${GREEN}Press Ctrl+C to exit${NC}"
      $compose_cmd logs -f "$service"
    else
      echo -e "${RED}Error: Service '$service' not found.${NC}"
      echo -e "${YELLOW}Available services:${NC}"
      $compose_cmd ps --services
      exit 1
    fi
  fi
}

# Show status
show_status() {
  echo -e "${YELLOW}Current status of Docker services:${NC}"
  $compose_cmd ps
}

# Pull AI model
pull_model() {
  echo -e "${YELLOW}Pulling AI model for Ollama...${NC}"
  echo -e "${BLUE}This may take a while depending on your internet connection.${NC}"
  $compose_cmd exec -T ollama ollama pull qwen2.5-7b-instruct
  echo -e "${GREEN}AI model pulled successfully.${NC}"
}

# Main command processing
case "$1" in
  start)
    start_docker
    ;;
  stop)
    stop_docker
    ;;
  restart)
    echo -e "${YELLOW}Restarting Docker environment...${NC}"
    stop_docker
    start_docker
    ;;
  logs)
    show_logs "$2"
    ;;
  ps)
    echo -e "${YELLOW}Listing running services:${NC}"
    $compose_cmd ps
    ;;
  build)
    echo -e "${YELLOW}Rebuilding Docker services...${NC}"
    $compose_cmd build
    echo -e "${GREEN}Build completed.${NC}"
    ;;
  pull)
    echo -e "${YELLOW}Pulling latest Docker images...${NC}"
    $compose_cmd pull
    echo -e "${GREEN}Images pulled successfully.${NC}"
    ;;
  clean)
    clean_docker
    ;;
  status)
    show_status
    ;;
  model)
    pull_model
    ;;
  help|--help|-h)
    show_help
    ;;
  "")
    # No arguments, show help
    show_help
    ;;
  *)
    echo -e "${RED}Unknown command: $1${NC}"
    echo -e "Use ${YELLOW}./docker-manage.sh help${NC} to see available commands."
    exit 1
    ;;
esac

exit 0 
