# Building from Windows with WSL

This guide explains how to build the Swarmonomicon project directly on your Windows machine using Windows Subsystem for Linux (WSL) and deploy it to an EC2 instance.

## Overview

If you've already cloned the repository on your Windows machine, you can build it directly using WSL without needing to transfer files from your Mac. This approach:

- Builds Linux binaries natively in WSL
- Works directly with your existing Windows code checkout
- Avoids the overhead of copying code between machines
- Prevents EC2 from being overtaxed during compilation

## Prerequisites

1. **Windows Setup Requirements**:
   - Windows 10/11 with WSL2 installed
   - Ubuntu distribution in WSL
   - Git Bash or similar bash shell for Windows

2. **SSH Configuration** (optional for deployment):
   - SSH access to your EC2 instance configured as `eaws` in your SSH config

## Quick Start

1. Clone the repository on Windows (if not already done)
2. Open Git Bash in the repository directory
3. Run the test script to verify your environment:
   ```bash
   ./test_windows_setup.sh
   ```
4. Build the project:
   ```bash
   ./build_from_windows.sh
   ```
5. Deploy to EC2 (optional):
   ```bash
   ./deploy_to_ec2.sh
   ```

## Available Scripts

We provide three scripts for building and deploying from Windows:

1. **test_windows_setup.sh**: Tests if your Windows WSL environment is properly set up
2. **build_from_windows.sh**: Builds the project directly using WSL
3. **deploy_to_ec2.sh**: Deploys the built binaries to your EC2 instance

## Testing Your Setup

Before building, run the test script to verify your Windows environment:

```bash
./test_windows_setup.sh
```

This script checks:
- WSL availability and Ubuntu installation
- Rust installation in WSL (will be installed automatically if missing)
- Access to your project directory from WSL
- Required dependencies in WSL
- SSH connectivity to your EC2 instance (optional)

## Building the Project

To build the project on Windows using WSL:

```bash
./build_from_windows.sh
```

This script:
- Uses WSL to build in a Linux environment
- Installs Rust in WSL if needed
- Installs required dependencies
- Builds the project with `cargo build --release`
- Copies binaries to a local `bin` directory
- Optionally deploys to EC2 if SSH access is configured

## Deploying to EC2

After building, you can deploy to EC2:

```bash
./deploy_to_ec2.sh
```

This script:
- Tests SSH connectivity to your EC2 instance
- Creates necessary directories on EC2
- Copies binaries to EC2
- Makes binaries executable
- Optionally restarts services using `restart_services.sh`

## Setting Up SSH for EC2 Deployment

To enable EC2 deployment, add this to your `~/.ssh/config` file:

```
Host eaws
    HostName your-ec2-instance-ip-or-dns
    User ubuntu
    IdentityFile ~/.ssh/your-ec2-key.pem
```

Replace placeholders with your actual EC2 information.

## Troubleshooting

### WSL Issues

If WSL is not working properly:

1. Install or update WSL:
   ```
   wsl --install
   ```

2. Make sure Ubuntu is installed:
   ```
   wsl --install -d Ubuntu
   ```

3. Check WSL status:
   ```
   wsl --status
   wsl -l -v
   ```

### Build Failures

If the build fails:

1. Check logs in the WSL environment:
   ```
   wsl -d Ubuntu
   cd /path/to/your/project
   cargo build --release -v
   ```

2. Make sure WSL can access your Windows files:
   ```
   wsl -d Ubuntu -e ls /mnt/c/
   ```

3. Check for dependency issues in WSL:
   ```
   wsl -d Ubuntu
   sudo apt update
   sudo apt install pkg-config libssl-dev libcurl4-openssl-dev cmake gcc g++ libc6-dev git
   ```

### Deployment Issues

If deployment to EC2 fails:

1. Verify SSH configuration:
   ```
   ssh eaws
   ```

2. Check bin directory exists:
   ```
   ls -la bin/
   ```

3. Ensure EC2 instance has necessary permissions:
   ```
   ssh eaws "mkdir -p ~/Swarmonomicon/bin && ls -la ~/Swarmonomicon"
   ``` 
