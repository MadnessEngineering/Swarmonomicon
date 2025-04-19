# Swarmonomicon Docker Setup for Windows
# This script will set up the Docker environment for Swarmonomicon on Windows

Write-Host "=== Swarmonomicon Docker Setup for Windows ===" -ForegroundColor Cyan
Write-Host "This script will set up the Docker environment for Swarmonomicon." -ForegroundColor Cyan
Write-Host ""

# Check if Docker is installed and running
try {
    docker info | Out-Null
    Write-Host "Docker is running." -ForegroundColor Green
} catch {
    Write-Host "Error: Docker is not installed or not running." -ForegroundColor Red
    Write-Host "Please install Docker Desktop for Windows and start it before running this script." -ForegroundColor Red
    exit 1
}

# Create necessary directories
Write-Host "Creating required directories..." -ForegroundColor Yellow
if (-not (Test-Path -Path "config")) {
    New-Item -Path "config" -ItemType Directory | Out-Null
}

# Check if we need to create mosquitto.conf
if (-not (Test-Path -Path "config\mosquitto.conf")) {
    Write-Host "Creating Mosquitto configuration..." -ForegroundColor Yellow
    $mosquittoConfig = @"
# Mosquitto MQTT Configuration for Swarmonomicon

# Basic configuration
persistence true
persistence_location /mosquitto/data/
log_dest file /mosquitto/log/mosquitto.log
log_dest stdout

# Default listener
listener 1883
protocol mqtt

# WebSockets listener for web clients
listener 9001
protocol websockets

# Allow anonymous connections with no authentication
# IMPORTANT: This is for development only
# For production, use password_file or another authentication method
allow_anonymous true
"@
    $mosquittoConfig | Out-File -FilePath "config\mosquitto.conf" -Encoding utf8
    Write-Host "Mosquitto configuration created." -ForegroundColor Green
}

# Model to use
$MODEL = "qwen2.5-7b-instruct"
Write-Host "Will configure to download model: $MODEL" -ForegroundColor Yellow

# Check WSL2 for optimal performance
$wsl_version = wsl --status | Select-String -Pattern "Default Version" -CaseSensitive
if ($wsl_version -match "2") {
    Write-Host "WSL2 detected. Docker should run optimally." -ForegroundColor Green
} else {
    Write-Host "Warning: WSL2 not detected or cannot be verified. Docker may not run optimally." -ForegroundColor Yellow
    Write-Host "Consider upgrading to WSL2 for better performance." -ForegroundColor Yellow
}

# Start the Docker containers
Write-Host "Starting Docker containers..." -ForegroundColor Yellow
docker-compose up -d

# Wait for Ollama to be ready
Write-Host "Waiting for Ollama service to be ready..." -ForegroundColor Yellow
$attempt = 0
$max_attempts = 30
$ready = $false

while ($attempt -lt $max_attempts -and -not $ready) {
    try {
        docker-compose exec -T ollama curl -sf http://localhost:11434/api/version | Out-Null
        $ready = $true
        Write-Host "Ollama is ready!" -ForegroundColor Green
    } catch {
        $attempt++
        Write-Host "Waiting for Ollama... (Attempt $attempt/$max_attempts)" -ForegroundColor Yellow
        Start-Sleep -Seconds 5
    }
}

if (-not $ready) {
    Write-Host "Warning: Ollama service didn't become ready in time. You may need to pull the models manually." -ForegroundColor Yellow
} else {
    # Pull the model
    Write-Host "Pulling the $MODEL model... (This may take a while depending on your internet connection)" -ForegroundColor Yellow
    docker-compose exec -T ollama ollama pull $MODEL
    Write-Host "Model pulling initiated. It may continue in the background." -ForegroundColor Green
}

# Check service health
Write-Host "Checking service health..." -ForegroundColor Yellow
docker-compose ps

Write-Host "=== Setup Complete ===" -ForegroundColor Green
Write-Host "Swarmonomicon is now running with Docker!" -ForegroundColor Green
Write-Host ""
Write-Host "Access the services at:" -ForegroundColor Cyan
Write-Host "- Web interface: http://localhost:3000" -ForegroundColor Cyan
Write-Host "- MQTT: localhost:1883 (or ws://localhost:9001 for WebSockets)" -ForegroundColor Cyan
Write-Host "- MongoDB: localhost:27017" -ForegroundColor Cyan
Write-Host ""
Write-Host "To see logs: docker-compose logs -f" -ForegroundColor Cyan
Write-Host "To stop all services: docker-compose down" -ForegroundColor Cyan
Write-Host ""
Write-Host "For more information, see DOCKER.md file." -ForegroundColor Cyan 
