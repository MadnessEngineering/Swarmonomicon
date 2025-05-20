# Cross-Compilation Guide for Swarmonomicon

This guide explains how to build the Swarmonomicon project on your local machine and deploy it to an EC2 instance.

## Prerequisites

- Docker installed on your local machine
- SSH access to your EC2 instance configured as `eaws` in your SSH config
- Git for version control

## Setup SSH Configuration

Make sure your SSH configuration has an entry for the `eaws` server. Add this to your `~/.ssh/config` file:

```
Host eaws
    HostName your-ec2-instance-ip-or-dns
    User ubuntu  # or your EC2 user
    IdentityFile ~/.ssh/your-ec2-key.pem
    ForwardAgent yes
```

Replace the placeholders with your actual EC2 information.

## Building and Deploying

The project includes two scripts that make the build and deployment process easy:

1. `build_and_deploy.sh`: Builds the project inside a Docker container and transfers the binaries to your EC2 instance
2. `restart_services.sh`: Restarts the services on your EC2 instance

### Steps

1. Make sure you're on the right branch with your changes
2. Run the build and deploy script:

```bash
./build_and_deploy.sh
```

This script will:
- Build a Docker image with the appropriate compilation environment
- Compile all binaries in the project
- Extract the compiled binaries
- Transfer them to your EC2 instance at `~/Swarmonomicon/`
- Optionally restart the services on your EC2 instance

### How It Works

The cross-compilation process uses Docker to ensure a consistent build environment:

1. A Dockerfile (`Dockerfile.build`) sets up a Rust environment on Debian Bullseye
2. All project dependencies are installed in the Docker container
3. The project is compiled with `cargo build --release`
4. The resulting binaries are extracted from the Docker container
5. These binaries are transferred to your EC2 instance via SCP

## Service Management

On the EC2 instance, the services can be managed in several ways:

1. **Systemd**: If systemd is available, the services will be managed as systemd units
2. **PM2**: If PM2 is available, the services will be managed as PM2 processes
3. **Background Processes**: As a fallback, the services will be started as background processes

## Troubleshooting

### SSH Connection Issues

If you encounter problems connecting to your EC2 instance:

```bash
ssh eaws
```

If this command doesn't work, check your SSH configuration.

### Docker Build Failures

If the Docker build fails, check:

1. Docker service is running
2. You have sufficient disk space
3. The Dockerfile.build file is correctly formatted

### Deployment Issues

If deployment fails:

1. Check that your EC2 instance is running
2. Verify your SSH configuration
3. Ensure the target directory on the EC2 instance is writable 
