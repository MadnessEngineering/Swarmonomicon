# EC2 Build Guide for Ubuntu/WSL

This guide explains how to build Swarmonomicon directly from Ubuntu (including WSL) and deploy it to an EC2 instance.

## Overview

This approach:
- Builds directly in your Ubuntu environment
- Creates statically linked Linux binaries
- Deploys them to your EC2 instance
- Ensures compatibility with your EC2 environment

## Prerequisites

- Ubuntu Linux (or WSL with Ubuntu)
- SSH access to your EC2 instance configured as `eaws`

## Quick Start

1. Run the build script from your Ubuntu terminal or WSL:
   ```bash
   ./ec2_build.sh
   ```

2. The script will:
   - Install necessary dependencies
   - Install or update Rust if needed
   - Build the project with `cargo build --release`
   - Collect binaries in a local `bin` directory
   - Optionally deploy to EC2 if you choose

## SSH Configuration

Before deployment, make sure you have the AWS EC2 instance configured in your SSH config.
Add the following to your `~/.ssh/config` file:

```
Host eaws
    HostName your-ec2-instance-ip-or-hostname
    User ubuntu
    IdentityFile ~/.ssh/your-ec2-key.pem
```

Replace the placeholders with your actual EC2 information.

## EC2 Server Setup

The deployment will place binaries in:
```
~/Swarmonomicon/bin/
```

This directory will be created automatically if it doesn't exist.

## Service Management

To restart services on the EC2 instance, the script will:
1. SSH into your EC2 instance
2. Change to the bin directory
3. Run the `restart_services.sh` script

If you need to customize service management, modify the `restart_services.sh` script.

## Troubleshooting

### SSH Connection Issues

If you encounter SSH connectivity problems:

1. Test direct SSH connectivity:
   ```bash
   ssh eaws
   ```

2. Verify the correct key permissions:
   ```bash
   chmod 600 ~/.ssh/your-ec2-key.pem
   ```

### Build Failures

If the build fails:

1. Make sure all dependencies are installed:
   ```bash
   sudo apt update
   sudo apt install pkg-config libssl-dev libcurl4-openssl-dev cmake gcc g++ libc6-dev git
   ```

2. Check Rust installation:
   ```bash
   rustc --version
   ```

3. Verify your Cargo.toml is valid:
   ```bash
   cargo check
   ```

### Deployment Issues

If deployment fails:

1. Check if the EC2 instance is running
2. Test SCP directly:
   ```bash
   scp -r bin/* eaws:~/test/
   ```
3. Verify network connectivity to your EC2 instance 
