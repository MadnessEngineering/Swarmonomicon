# Swarmonomicon Docker Guide

This guide explains how to run Swarmonomicon using Docker, which provides a consistent environment across different operating systems including macOS and Windows.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) installed on your system
- [Docker Compose](https://docs.docker.com/compose/install/) installed on your system
- Basic understanding of Docker and command line usage

## Quick Start

We provide setup scripts for easy initialization:

### macOS/Linux

```bash
# Make the script executable
chmod +x docker-setup.sh

# Run the setup script
./docker-setup.sh
```

### Windows

Open PowerShell and run:

```powershell
# You may need to set execution policy
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Run the setup script
.\docker-setup.ps1
```

The setup scripts will create necessary directories, configure the environment, and start the Docker containers.

## Manual Setup

If you prefer to set up manually or the scripts don't work for your environment:

1. Create necessary directories:
   ```bash
   mkdir -p data models mosquitto/config mosquitto/data mosquitto/log
   ```

2. Create Mosquitto MQTT config:
   ```bash
   echo "listener 1883" > mosquitto/config/mosquitto.conf
   echo "allow_anonymous true" >> mosquitto/config/mosquitto.conf
   echo "persistence true" >> mosquitto/config/mosquitto.conf
   echo "persistence_location /mosquitto/data/" >> mosquitto/config/mosquitto.conf
   echo "log_dest file /mosquitto/log/mosquitto.log" >> mosquitto/config/mosquitto.conf
   ```

3. Create `.env` file with your OpenAI API key:
   ```bash
   echo "OPENAI_API_KEY=your_api_key_here" > .env
   ```

4. Start the containers:
   ```bash
   # For macOS
   docker-compose --profile macos up -d

   # For Windows
   docker-compose --profile windows up -d
   
   # To include RL capabilities, add the rl profile
   docker-compose --profile macos --profile rl up -d
   ```

## Service Architecture

The Docker setup includes several interconnected services:

- **swarm**: Main Swarmonomicon service
- **todo_worker**: Worker service for processing tasks
- **mcp_todo_server**: MCP Todo server for task management
- **mongodb**: Database for storing agent data
- **mqtt**: MQTT broker for inter-agent communication
- **train_flappy**: Optional service for reinforcement learning (RL) training

## Accessing Services

- **Main Web Interface**: [http://localhost:8080](http://localhost:8080)
- **MCP Todo Server**: [http://localhost:8081](http://localhost:8081)
- **MQTT Broker**: localhost:1883

## Common Commands

```bash
# Start all services
docker-compose up -d

# Start with platform-specific optimizations
docker-compose --profile macos up -d  # For macOS
docker-compose --profile windows up -d  # For Windows

# Enable RL features
docker-compose --profile rl up -d

# View logs
docker-compose logs -f

# View logs for a specific service
docker-compose logs -f swarm

# Stop all services
docker-compose down

# Rebuild containers (after code changes)
docker-compose build
```

## Data Persistence

The following directories are mounted for data persistence:

- `./data:/app/data`: Application data
- `./models:/app/models`: ML/RL models and training data
- `./mosquitto/data:/mosquitto/data`: MQTT broker data
- `./mosquitto/log:/mosquitto/log`: MQTT logs

## Troubleshooting

### Common Issues

1. **Port conflicts**: If services fail to start due to port conflicts, modify the port mappings in `docker-compose.yml`.

2. **Docker memory issues**: For resource-intensive operations like RL training, you may need to allocate more memory to Docker:
   - On Docker Desktop, go to Settings > Resources > Advanced > Memory

3. **Volume permission issues**: If you encounter permission problems with mounted volumes:
   ```bash
   # Fix permissions
   chmod -R 777 data models mosquitto
   ```

4. **OpenAI API errors**: Ensure your API key is correctly set in the `.env` file.

## Building Your Own Image

To build a custom image with your modifications:

```bash
# Build all services
docker-compose build

# Build a specific service
docker-compose build swarm
```

## Advanced Configuration

The `docker-compose.yml` file contains various configuration options. Common changes include:

- Modifying port mappings
- Changing volume mounts
- Adjusting environment variables
- Modifying resource constraints

## For Developers

When developing and making code changes:

1. Edit the source code as normal
2. Rebuild the Docker image: `docker-compose build`
3. Restart the services: `docker-compose up -d`

For faster development iterations, you can mount your source code directly:

```yaml
volumes:
  - ./src:/app/src
```

Add this to the service definition in `docker-compose.yml` for services you're actively developing.

## RL Training

For reinforcement learning training:

```bash
# Start the training service
docker-compose --profile rl up -d train_flappy

# Monitor the training logs
docker-compose logs -f train_flappy
```

Training data and models are saved to the `./models` directory.

---

## License

This Docker setup is part of the Swarmonomicon project and follows the same licensing terms. See the main LICENSE file for details. 
