#!/bin/bash

# Check if Docker mode is enabled
USE_DOCKER=${USE_DOCKER:-false}

# Check if local Ollama is running using the check_service script
./check_service.sh "Ollama" "localhost" "11434"

if [ $? -eq 0 ]; then
    if [ "$USE_DOCKER" = true ]; then
        echo "Local Ollama is running. Using host.docker.internal for Docker containers."
    else
        echo "Local Ollama is running. Will connect to localhost:11434."
    fi
    exit 0
else
    echo "Warning: Local Ollama is not running. Please start it with 'ollama serve' before starting the services."
    exit 1
fi 
