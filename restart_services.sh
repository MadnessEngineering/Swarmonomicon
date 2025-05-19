#!/bin/bash

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Restarting Swarmonomicon services...${NC}"

# Stop any running instances using systemctl or pm2
if command -v systemctl &> /dev/null; then
    echo -e "${YELLOW}Using systemctl to restart services...${NC}"
    
    # Check if services exist
    if systemctl list-unit-files | grep -q swarmonomicon; then
        sudo systemctl restart swarmonomicon*
    else
        echo -e "${RED}No systemd services found for Swarmonomicon.${NC}"
        echo -e "${YELLOW}You may need to set up systemd services manually.${NC}"
    fi
elif command -v pm2 &> /dev/null; then
    echo -e "${YELLOW}Using PM2 to restart services...${NC}"
    
    # Check if services exist in PM2
    if pm2 list | grep -q swarm; then
        pm2 restart all
    else
        echo -e "${RED}No PM2 services found for Swarmonomicon.${NC}"
        echo -e "${YELLOW}Setting up PM2 services...${NC}"
        
        # Create PM2 ecosystem file if it doesn't exist
        if [ ! -f ecosystem.config.js ]; then
            cat > ecosystem.config.js << 'EOL'
module.exports = {
  apps: [
    {
      name: "swarm",
      script: "./swarm",
      instances: 1,
      autorestart: true,
      watch: false,
    },
    {
      name: "todo_worker",
      script: "./todo_worker",
      instances: 1,
      autorestart: true,
      watch: false,
    },
    {
      name: "mcp_todo_server",
      script: "./mcp_todo_server",
      instances: 1,
      autorestart: true,
      watch: false,
    }
  ]
};
EOL
        fi
        
        # Start PM2 services
        pm2 start ecosystem.config.js
    fi
else
    echo -e "${RED}Neither systemctl nor pm2 found. Please install one of them to manage services.${NC}"
    echo -e "${YELLOW}For now, starting services in the background...${NC}"
    
    # Start services manually
    ./swarm > swarm.log 2>&1 &
    ./todo_worker > todo_worker.log 2>&1 &
    ./mcp_todo_server > mcp_todo_server.log 2>&1 &
    
    echo -e "${GREEN}Services started in background. Check logs for details.${NC}"
fi

echo -e "${GREEN}Services restarted!${NC}" 
