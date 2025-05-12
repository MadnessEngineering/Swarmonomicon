#!/bin/bash

# First check if Ollama is running
./check_ollama.sh
if [ $? -ne 0 ]; then
    exit 1
fi

# Run the services with docker-compose
echo "Starting Swarmonomicon services with docker-compose..."
docker compose up "$@" 
