# Docker Deployment Guide for Swarmonomicon

This guide provides instructions for deploying the Swarmonomicon project using Docker on macOS (Intel and Apple Silicon) and Windows platforms.

## Prerequisites

- Docker Desktop installed (latest version recommended)
- At least 8GB of RAM allocated to Docker
- 20GB of free disk space (for Docker images and volumes)
- For GPU acceleration (optional): 
  - On Windows: NVIDIA GPU with [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html) installed
  - On macOS: Apple Silicon M1/M2/M3 chips for experimental GPU acceleration

## Quick Start

1. Clone the repository:
   ```bash
   git clone https://github.com/DanEdens/Swarmonomicon.git
   cd Swarmonomicon
   ```

2. Run the setup script:
   ```bash
   # For macOS/Linux
   ./docker-setup.sh
   
   # For Windows
   .\docker-setup.ps1
   ```

3. Start the services:
   ```bash
   ./start-docker.sh
   ```

4. Access the services:
   - Web interface: http://localhost:3000
   - MQTT: localhost:1883 (or ws://localhost:9001 for WebSockets)
   - MongoDB: localhost:27017
   - Ollama API: http://localhost:11434/api

5. Pull the required AI model:
   ```bash
   docker compose exec ollama ollama pull qwen2.5-7b-instruct
   ```

## Configuration

### Environment Variables

You can customize the deployment by modifying these environment variables in the `docker-compose.yml` file:

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (info, debug, trace) | info |
| `AI_ENDPOINT` | LLM API endpoint | http://ollama:11434/api/generate |
| `AI_MODEL` | Name of the LLM model | qwen2.5-7b-instruct |
| `RTK_MONGO_URI` | MongoDB connection URI | mongodb://mongodb:27017 |
| `RTK_MONGO_DB` | MongoDB database name | swarmonomicon |
| `MQTT_HOST` | MQTT broker hostname | mosquitto |
| `MQTT_PORT` | MQTT broker port | 1883 |
| `TODO_CHECK_INTERVAL_SECS` | Todo checking interval | 30 |

### Volumes

The Docker setup uses these persistent volumes:

- `mongodb_data`: MongoDB data
- `mosquitto_data`: MQTT broker persistent messages
- `mosquitto_log`: MQTT broker logs
- `ollama_models`: AI models for Ollama

## Resource Management

The Docker Compose file includes resource limits for each service:

| Service | Memory Limit | CPU Limit |
|---------|--------------|-----------|
| MongoDB | 1GB | 1 core |
| Mosquitto | 256MB | 0.5 core |
| Ollama | 8GB | 4 cores |
| Swarm | 1GB | 1 core |
| Todo Worker | 512MB | 0.5 core |
| MCP Todo Server | 512MB | 0.5 core |

You can adjust these limits in the `docker-compose.yml` file based on your system's capabilities.

## Platform-Specific Notes

### macOS

- The Docker image runs natively on both Intel and Apple Silicon (M1/M2/M3) Macs
- For Apple Silicon, the setup script automatically configures the environment
- Performance may vary based on machine specifications

### Windows

- Make sure WSL2 is properly configured for best performance
- For NVIDIA GPU support, update to the latest NVIDIA drivers and install NVIDIA Container Toolkit
- The Docker bridge network requires allowing connections through Windows Firewall

## Helper Scripts

The setup creates these helper scripts:

- `start-docker.sh` - Start the Docker environment
- `stop-docker.sh` - Stop the Docker environment
- `view-logs.sh` - View logs from containers (use with service name to see specific logs)

## Troubleshooting

### Common Issues

1. **Ollama model download issues**:
   ```bash
   ./view-logs.sh ollama
   ```
   If you see disk space or network errors, try downloading directly:
   ```bash
   docker compose exec ollama ollama pull qwen2.5-7b-instruct
   ```

2. **MongoDB connection failures**:
   Check if MongoDB is healthy:
   ```bash
   docker compose ps mongodb
   ```
   Verify the container is "healthy" in the status column.

3. **MQTT connectivity problems**:
   Test the MQTT broker connectivity:
   ```bash
   docker run --rm -it eclipse-mosquitto mosquitto_sub -h mosquitto -t test
   ```
   In another terminal:
   ```bash
   docker run --rm -it eclipse-mosquitto mosquitto_pub -h mosquitto -t test -m "hello"
   ```

4. **Container resource limits**:
   If containers are crashing due to insufficient resources, increase the memory allocation in the `docker-compose.yml` file.

## Production Deployment

For production environments:

1. Enable authentication for Mosquitto (modify `mosquitto.conf`)
2. Configure MongoDB authentication
3. Set up TLS for secure connections
4. Consider using Docker Swarm or Kubernetes for high availability
5. Add monitoring with Prometheus and Grafana
6. Implement proper backup strategies for all persistent volumes

## Updating the Application

To update to a new version:

1. Pull the latest code:
   ```bash
   git pull
   ```

2. Rebuild and restart the containers:
   ```bash
   ./stop-docker.sh
   docker compose build
   ./start-docker.sh
   ```

## Health Checks

The Docker setup includes health checks for all services. To view the current health status:

```bash
docker compose ps
```

For detailed health check logs:

```bash
docker inspect --format "{{json .State.Health }}" swarmonomicon_mongodb_1 | jq
```

## License

This project is licensed under the MIT License - see the LICENSE file for details. 
