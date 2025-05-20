# macOS to EC2 Cross-Compilation Guide

This guide explains how to build the Swarmonomicon project on macOS and deploy it to an EC2 Linux instance.

## Overview

Cross-compiling from macOS to Linux offers several advantages:
- Builds Linux binaries on your local macOS machine
- Avoids taxing the EC2 instance during compilation
- Produces statically linked binaries that work reliably on the target system

## Prerequisites

1. **macOS Environment**:
   - macOS (tested on Ventura/Sonoma)
   - Homebrew package manager
   - Rust and Cargo installed

2. **SSH Configuration**:
   - SSH access to your EC2 instance configured as `eaws` in your SSH config

## Setup Steps

### 1. Install Required Tools

The build script will check for and install these automatically, but you can also install them manually:

```bash
# Install musl cross compiler
brew install FiloSottile/musl-cross/musl-cross

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the Linux musl target
rustup target add x86_64-unknown-linux-musl
```

### 2. Configure SSH

Add this to your `~/.ssh/config` file:

```
Host eaws
    HostName your-ec2-instance-ip-or-hostname
    User ubuntu
    IdentityFile ~/.ssh/your-key.pem
```

Replace the placeholders with your actual EC2 information.

## Building and Deploying

Run the build script:

```bash
./build_macos_to_ec2.sh
```

This script will:
1. Check and install required dependencies
2. Build the project targeting Linux
3. Copy the binaries to a local `bin` directory
4. Optionally deploy them to your EC2 instance
5. Optionally restart services on your EC2 instance

## How It Works

The cross-compilation process works as follows:

1. The Rust compiler uses the musl-cross toolchain to build Linux binaries
2. Static linking is enabled to avoid dependency issues on the target
3. Binaries are found and copied to a staging directory
4. SCP transfers the binaries to the EC2 instance
5. Services can be restarted remotely

## Troubleshooting

### Build Failures

If the build fails:

1. Check the error messages for missing dependencies
2. Ensure your `.cargo/config.toml` is properly configured
3. Try building a simple test program first to verify cross-compilation works

### Deployment Issues

If deployment fails:

1. Test your SSH connection: `ssh eaws`
2. Check permissions on the target directory
3. Verify firewall settings allow SSH/SCP

### Runtime Issues

If binaries run on your EC2 instance but crash:

1. Check for dynamic linking issues: `ldd your_binary`
2. Verify the binaries are executable: `chmod +x your_binary`
3. Check system resources (memory, disk space)
4. Examine logs for specific error messages

## Next Steps

After successful deployment, you can:

1. Set up systemd services for automatic startup
2. Configure monitoring for your services
3. Set up log rotation for the log files 
