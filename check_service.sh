#!/bin/bash

# Usage: ./check_service.sh <service_name> <host> <port> [timeout_seconds]
# Returns: 0 if service is available, 1 if not

SERVICE_NAME=$1
HOST=$2
PORT=$3
TIMEOUT=${4:-2}  # Default timeout is 2 seconds

if [ -z "$SERVICE_NAME" ] || [ -z "$HOST" ] || [ -z "$PORT" ]; then
    echo "Usage: $0 <service_name> <host> <port> [timeout_seconds]"
    exit 2
fi

# Check if nc (netcat) is available
if ! command -v nc &> /dev/null; then
    echo "Error: netcat (nc) is required but not installed."
    echo "Please install it with 'brew install netcat' on macOS or 'apt-get install netcat' on Linux."
    exit 3
fi

echo "Checking if $SERVICE_NAME is available at $HOST:$PORT..."
nc -z -w $TIMEOUT $HOST $PORT

if [ $? -eq 0 ]; then
    echo "$SERVICE_NAME is available at $HOST:$PORT."
    exit 0
else
    echo "$SERVICE_NAME is not available at $HOST:$PORT."
    exit 1
fi 
