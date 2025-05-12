#!/bin/bash

# Check if local Ollama is running
if curl -s http://localhost:11434/api/version > /dev/null; then
    echo "Local Ollama is running. Using host.docker.internal to connect from Docker."
else
    echo "Warning: Local Ollama is not running. Please start it with 'ollama serve' before starting the services."
    exit 1
fi 
