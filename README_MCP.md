# MCP Todo Server

## Setup and Configuration

The MCP Todo Server can be run in multiple ways depending on your needs:

### Prerequisites

- Ollama must be running locally (http://localhost:11434)
- MongoDB and MQTT broker (either remote or local via Docker)

### Quick Start

To run the MCP Todo Server with automatic service detection:

```bash
# Start the server, auto-detecting available services
./run_mcp.sh

# Force using local Docker services instead of remote ones
./run_mcp.sh --force-local

# Run the server through Docker instead of locally
./run_mcp.sh --docker

# Specify a remote server for MongoDB and MQTT
./run_mcp.sh --awsip=<AWS_IP_ADDRESS>
```

The script will automatically:
1. Check if Ollama is running locally
2. Check if MongoDB and MQTT are available at the specified IP address
3. Start any missing services using Docker
4. Launch the MCP Todo Server with the appropriate connections

### Docker Compose Profiles

The docker-compose.yml uses profiles to make services optional:

```bash
# Run only the MCP Todo Server (uses remote services)
docker compose up

# Run MCP Todo Server with local MongoDB
docker compose --profile mongodb up

# Run MCP Todo Server with local MQTT
docker compose --profile mqtt up

# Run the complete stack with all services
docker compose --profile all up
```

### Environment Variables

To customize connections, set these environment variables before running:

```bash
# Set a different AWS IP address
export AWSIP=<AWS_IP_ADDRESS>

# Set a different MongoDB port
export AWSPORT=27017

# Set MongoDB connection URI directly
export RTK_MONGO_URI=mongodb://localhost:27017

# Set MQTT connection details
export MQTT_HOST=localhost
export MQTT_PORT=1883
```

## Utility Scripts

The repository includes several utility scripts:

- `check_service.sh`: General utility to check if a service is available at a given host/port
- `check_ollama.sh`: Verifies if Ollama is running locally
- `run_mcp.sh`: Main script to run the MCP Todo Server with automatic service detection 
