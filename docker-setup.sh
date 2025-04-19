#!/bin/bash
# Swarmonomicon Docker Setup Script
# This script helps set up the Docker environment for Swarmonomicon

set -e  # Exit on error

# Detect OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="macos"
    echo "🍎 macOS detected"
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    PLATFORM="windows"
    echo "🪟 Windows detected"
else
    PLATFORM="linux"
    echo "🐧 Linux detected"
fi

# Print banner
echo "=================================================="
echo "🧙 Swarmonomicon Docker Setup"
echo "=================================================="
echo "This script will set up your Docker environment for Swarmonomicon."
echo

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    echo "   Visit https://docs.docker.com/get-docker/ for installation instructions."
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    echo "   Visit https://docs.docker.com/compose/install/ for installation instructions."
    exit 1
fi

echo "✅ Docker and Docker Compose are installed."
echo

# Create necessary directories
echo "Creating necessary directories..."
mkdir -p data models mosquitto/config mosquitto/data mosquitto/log

# Create Mosquitto configuration
echo "Configuring Mosquitto MQTT broker..."
cat > mosquitto/config/mosquitto.conf << EOL
listener 1883
allow_anonymous true
persistence true
persistence_location /mosquitto/data/
log_dest file /mosquitto/log/mosquitto.log
EOL

# Check for OpenAI API key
if [ -z "$OPENAI_API_KEY" ]; then
    echo "⚠️  OpenAI API key not found in environment."
    echo "   Some features may not work without it."
    
    read -p "Would you like to enter an OpenAI API key now? (y/n): " SET_KEY
    
    if [[ "$SET_KEY" == "y" || "$SET_KEY" == "Y" ]]; then
        read -p "Enter your OpenAI API key: " API_KEY
        echo "OPENAI_API_KEY=$API_KEY" > .env
        echo "✅ API key saved to .env file."
    else
        echo "OPENAI_API_KEY=" > .env
        echo "⚠️  No API key set. You can edit the .env file later to add it."
    fi
else
    echo "OPENAI_API_KEY=$OPENAI_API_KEY" > .env
    echo "✅ Using OpenAI API key from environment."
fi

# Check for RL features
read -p "Do you want to enable Reinforcement Learning features? (y/n): " ENABLE_RL

# Show setup summary
echo
echo "=================================================="
echo "🚀 Setup Summary"
echo "=================================================="
echo "Platform detected: $PLATFORM"
echo "OpenAI API key: ${OPENAI_API_KEY:0:4}... (${#OPENAI_API_KEY} chars)"
echo "RL features: ${ENABLE_RL}"
echo "Directories created:"
echo "  - ./data"
echo "  - ./models"
echo "  - ./mosquitto/config"
echo "  - ./mosquitto/data"
echo "  - ./mosquitto/log"
echo

# Start services
echo "Starting services..."

if [[ "$ENABLE_RL" == "y" || "$ENABLE_RL" == "Y" ]]; then
    PROFILES="--profile $PLATFORM --profile rl"
else
    PROFILES="--profile $PLATFORM"
fi

# On Windows, use winpty if available
if [[ "$PLATFORM" == "windows" && -x "$(command -v winpty)" ]]; then
    winpty docker-compose up -d $PROFILES
else
    docker-compose up -d $PROFILES
fi

echo
echo "✅ Services started successfully!"
echo
echo "=================================================="
echo "📋 Usage Instructions"
echo "=================================================="
echo "To start all services:              docker-compose up -d"
echo "To start specific service:          docker-compose up -d swarm"
echo "To start RL training:               docker-compose --profile rl up -d"
echo "To view logs:                       docker-compose logs -f"
echo "To stop all services:               docker-compose down"
echo "To rebuild (after code changes):    docker-compose build"
echo
echo "Access web interface:               http://localhost:8080"
echo "Access MCP Todo server:             http://localhost:8081"
echo "MQTT broker:                        localhost:1883"
echo
echo "Directories mounted:"
echo "  - ./data:/app/data (persistent data)"
echo "  - ./models:/app/models (RL models)"
echo
echo "Happy coding! 🧙‍♂️"
