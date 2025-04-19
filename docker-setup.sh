#!/bin/bash
# Setup script for Swarmonomicon Docker environment on macOS/Linux

set -e

echo "=== Swarmonomicon Docker Setup ==="
echo "This script will set up the Docker environment for Swarmonomicon."

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed. Please install Docker first."
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "Error: Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Check if Docker is running
if ! docker info &> /dev/null; then
    echo "Error: Docker is not running. Please start Docker first."
    exit 1
fi

# Create necessary directories
echo "Creating required directories..."
mkdir -p config

# Check if we need to create mosquitto.conf
if [ ! -f "config/mosquitto.conf" ]; then
    echo "Creating Mosquitto configuration..."
    cat > config/mosquitto.conf << EOF
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
    echo "Mosquitto configuration created."
fi

# Check for models.txt and download required models
echo "Checking if Ollama models need to be pre-downloaded..."
MODEL="qwen2.5-7b-instruct"
echo "Will configure to download model: $MODEL"

# Start the Docker containers
echo "Starting Docker containers..."
docker-compose up -d

# Wait for Ollama to be ready
echo "Waiting for Ollama service to be ready..."
attempt=0
max_attempts=30
while [ $attempt -lt $max_attempts ]; do
    if docker-compose exec -T ollama curl -sf http://localhost:11434/api/version &> /dev/null; then
        echo "Ollama is ready!"
        break
    fi
    attempt=$((attempt+1))
    echo "Waiting for Ollama... (Attempt $attempt/$max_attempts)"
    sleep 5
done

if [ $attempt -eq $max_attempts ]; then
    echo "Warning: Ollama service didn't become ready in time. You may need to pull the models manually."
else
    # Pull the model
    echo "Pulling the $MODEL model... (This may take a while depending on your internet connection)"
    docker-compose exec -T ollama ollama pull $MODEL
    echo "Model pulling initiated. It may continue in the background."
fi

echo "Checking service health..."
docker-compose ps

echo "=== Setup Complete ==="
echo "Swarmonomicon is now running with Docker!"
echo ""
echo "Access the services at:"
echo "- Web interface: http://localhost:3000"
echo "- MQTT: localhost:1883 (or ws://localhost:9001 for WebSockets)"
echo "- MongoDB: localhost:27017"
echo ""
echo "To see logs: docker-compose logs -f"
echo "To stop all services: docker-compose down"
echo ""
echo "For more information, see DOCKER.md file."
