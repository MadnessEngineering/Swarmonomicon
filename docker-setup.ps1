# Swarmonomicon Docker Setup Script for Windows
# This script helps set up the Docker environment for Swarmonomicon on Windows systems

# Print banner
Write-Host "=================================================="
Write-Host "🧙 Swarmonomicon Docker Setup (Windows)" -ForegroundColor Cyan
Write-Host "=================================================="
Write-Host "This script will set up your Docker environment for Swarmonomicon."
Write-Host ""

# Check if Docker is installed
try {
    $dockerVersion = docker --version
    Write-Host "✅ Docker detected: $dockerVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker is not installed or not in PATH. Please install Docker Desktop for Windows." -ForegroundColor Red
    Write-Host "   Visit https://docs.docker.com/desktop/windows/install/ for installation instructions."
    exit 1
}

# Check if Docker Compose is installed
try {
    $composeVersion = docker-compose --version
    Write-Host "✅ Docker Compose detected: $composeVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker Compose is not installed or not in PATH." -ForegroundColor Red
    Write-Host "   It should be included with Docker Desktop for Windows."
    exit 1
}

Write-Host ""

# Create necessary directories
Write-Host "Creating necessary directories..." -ForegroundColor Yellow
New-Item -Path "data" -ItemType Directory -Force | Out-Null
New-Item -Path "models" -ItemType Directory -Force | Out-Null
New-Item -Path "mosquitto\config" -ItemType Directory -Force | Out-Null
New-Item -Path "mosquitto\data" -ItemType Directory -Force | Out-Null
New-Item -Path "mosquitto\log" -ItemType Directory -Force | Out-Null

# Create Mosquitto configuration
Write-Host "Configuring Mosquitto MQTT broker..." -ForegroundColor Yellow
@"
listener 1883
allow_anonymous true
persistence true
persistence_location /mosquitto/data/
log_dest file /mosquitto/log/mosquitto.log
"@ | Out-File -FilePath "mosquitto\config\mosquitto.conf" -Encoding ASCII

# Check for OpenAI API key
$apiKey = [Environment]::GetEnvironmentVariable("OPENAI_API_KEY", "User")
if ([string]::IsNullOrEmpty($apiKey)) {
    Write-Host "⚠️ OpenAI API key not found in environment." -ForegroundColor Yellow
    Write-Host "   Some features may not work without it."
    
    $setKey = Read-Host "Would you like to enter an OpenAI API key now? (y/n)"
    
    if ($setKey -eq "y" -or $setKey -eq "Y") {
        $apiKey = Read-Host "Enter your OpenAI API key"
        "OPENAI_API_KEY=$apiKey" | Out-File -FilePath ".env" -Encoding ASCII
        Write-Host "✅ API key saved to .env file." -ForegroundColor Green
    } else {
        "OPENAI_API_KEY=" | Out-File -FilePath ".env" -Encoding ASCII
        Write-Host "⚠️ No API key set. You can edit the .env file later to add it." -ForegroundColor Yellow
    }
} else {
    "OPENAI_API_KEY=$apiKey" | Out-File -FilePath ".env" -Encoding ASCII
    Write-Host "✅ Using OpenAI API key from environment." -ForegroundColor Green
}

# Check for RL features
$enableRL = Read-Host "Do you want to enable Reinforcement Learning features? (y/n)"

# Show setup summary
Write-Host ""
Write-Host "=================================================="
Write-Host "🚀 Setup Summary" -ForegroundColor Cyan
Write-Host "=================================================="
Write-Host "Platform: Windows"
if ($apiKey) {
    $firstFour = $apiKey.Substring(0, [Math]::Min(4, $apiKey.Length))
    Write-Host "OpenAI API key: $firstFour... ($($apiKey.Length) chars)"
} else {
    Write-Host "OpenAI API key: Not set"
}
Write-Host "RL features: $enableRL"
Write-Host "Directories created:"
Write-Host "  - .\data"
Write-Host "  - .\models"
Write-Host "  - .\mosquitto\config"
Write-Host "  - .\mosquitto\data"
Write-Host "  - .\mosquitto\log"
Write-Host ""

# Start services
Write-Host "Starting services..." -ForegroundColor Yellow

if ($enableRL -eq "y" -or $enableRL -eq "Y") {
    $profiles = "--profile windows --profile rl"
} else {
    $profiles = "--profile windows"
}

# Start Docker Compose
try {
    $command = "docker-compose up -d $profiles"
    Write-Host "Running: $command" -ForegroundColor DarkGray
    Invoke-Expression $command
    
    Write-Host ""
    Write-Host "✅ Services started successfully!" -ForegroundColor Green
} catch {
    Write-Host "❌ Error starting services: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "=================================================="
Write-Host "📋 Usage Instructions" -ForegroundColor Cyan
Write-Host "=================================================="
Write-Host "To start all services:              docker-compose up -d"
Write-Host "To start specific service:          docker-compose up -d swarm"
Write-Host "To start RL training:               docker-compose --profile rl up -d"
Write-Host "To view logs:                       docker-compose logs -f"
Write-Host "To stop all services:               docker-compose down"
Write-Host "To rebuild (after code changes):    docker-compose build"
Write-Host ""
Write-Host "Access web interface:               http://localhost:8080"
Write-Host "Access MCP Todo server:             http://localhost:8081"
Write-Host "MQTT broker:                        localhost:1883"
Write-Host ""
Write-Host "Directories mounted:"
Write-Host "  - .\data:/app/data (persistent data)"
Write-Host "  - .\models:/app/models (RL models)"
Write-Host ""
Write-Host "Happy coding! 🧙‍♂️" -ForegroundColor Magenta 
