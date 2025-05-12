#!/bin/bash

# Check if local Ollama is running using the check_service script
./check_service.sh "Ollama" "localhost" "11434"

if [ $? -eq 0 ]; then
    echo "Local Ollama is running. Using host.docker.internal to connect from Docker."
    exit 0
else
    echo "Warning: Local Ollama is not running. Please start it with 'ollama serve' before starting the services."
    exit 1
fi 
