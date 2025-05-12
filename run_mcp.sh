#!/bin/bash

# Default AWS settings
AWSIP=${AWSIP:-"AWS_IP_ADDRESS"}  # Replace with actual IP at runtime
AWSPORT=${AWSPORT:-27017}
MQTT_REMOTE_PORT=${MQTT_REMOTE_PORT:-1883}
FORCE_LOCAL=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --force-local)
      FORCE_LOCAL=true
      shift
      ;;
    --awsip=*)
      AWSIP="${1#*=}"
      shift
      ;;
    --awsport=*)
      AWSPORT="${1#*=}"
      shift
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: $0 [--force-local] [--awsip=IP] [--awsport=PORT]"
      exit 1
      ;;
  esac
done

# First check if Ollama is running
./check_ollama.sh
if [ $? -ne 0 ]; then
    exit 1
fi

# Check if MongoDB is available remotely
USE_LOCAL_MONGO=true
if [ "$FORCE_LOCAL" = false ]; then
    ./check_service.sh "MongoDB" "$AWSIP" "$AWSPORT"
    if [ $? -eq 0 ]; then
        echo "Remote MongoDB found at $AWSIP:$AWSPORT. Using remote instance."
        USE_LOCAL_MONGO=false
        MONGO_URI="mongodb://$AWSIP:$AWSPORT"
    else
        echo "Remote MongoDB not available. Will use local Docker instance."
        MONGO_URI="mongodb://localhost:27017"
    fi
else
    echo "Forcing local MongoDB usage."
    MONGO_URI="mongodb://localhost:27017"
fi

# Check if MQTT is available remotely
USE_LOCAL_MQTT=true
if [ "$FORCE_LOCAL" = false ]; then
    ./check_service.sh "MQTT" "$AWSIP" "$MQTT_REMOTE_PORT"
    if [ $? -eq 0 ]; then
        echo "Remote MQTT found at $AWSIP:$MQTT_REMOTE_PORT. Using remote instance."
        USE_LOCAL_MQTT=false
        MQTT_HOST=$AWSIP
        MQTT_PORT=$MQTT_REMOTE_PORT
    else
        echo "Remote MQTT not available. Will use local Docker instance."
        MQTT_HOST="localhost"
        MQTT_PORT="1883"
    fi
else
    echo "Forcing local MQTT usage."
    MQTT_HOST="localhost"
    MQTT_PORT="1883"
fi

# Start required services in Docker if needed
if [ "$USE_LOCAL_MONGO" = true ] && [ "$USE_LOCAL_MQTT" = true ]; then
    echo "Starting MongoDB and MQTT in Docker..."
    docker compose up -d mongodb mosquitto
elif [ "$USE_LOCAL_MONGO" = true ]; then
    echo "Starting MongoDB in Docker..."
    docker compose up -d mongodb
elif [ "$USE_LOCAL_MQTT" = true ]; then
    echo "Starting MQTT in Docker..."
    docker compose up -d mosquitto
else
    echo "Using remote services, no need to start Docker containers."
fi

# Wait for local services to be healthy if any were started
if [ "$USE_LOCAL_MONGO" = true ] || [ "$USE_LOCAL_MQTT" = true ]; then
    echo "Waiting for Docker services to be healthy..."
    # Initial delay to give services time to register
    sleep 5
    docker compose ps | grep "(healthy)" > /dev/null
    while [ $? -ne 0 ]; do
        echo "Waiting for services to become healthy..."
        sleep 5
        docker compose ps | grep "(healthy)" > /dev/null
    done
    echo "Docker services are healthy."
fi

echo "Starting MCP Todo Server..."

# Run the MCP Todo Server with the determined connection parameters
RUST_LOG=info \
AWSIP=$AWSIP \
AWSPORT=$AWSPORT \
AI_ENDPOINT=http://localhost:11434/api/generate \
AI_MODEL=qwen2.5-7b-instruct \
RTK_MONGO_URI=$MONGO_URI \
RTK_MONGO_DB=swarmonomicon \
MQTT_HOST=$MQTT_HOST \
MQTT_PORT=$MQTT_PORT \
./target/release/mcp_todo_server
