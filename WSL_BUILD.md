# WSL-Based Cross-Compilation for Swarmonomicon

This guide explains how to use Windows Subsystem for Linux (WSL) on a Windows machine to build the Swarmonomicon project and deploy it to an EC2 instance.

## Overview

Using WSL for cross-compilation provides several advantages:
- Builds Linux binaries natively in a Linux environment
- Avoids the heavy load on EC2 instances that might crash during compilation
- Doesn't require setting up cross-compilation toolchains on macOS

## Prerequisites

1. **SSH Configuration**:
   - SSH access to your Windows machine configured as `salad` in your SSH config
   - SSH access to your EC2 instance configured as `eaws` in your SSH config

2. **Windows Machine Setup**:
   - Windows 10/11 with WSL2 installed
   - Ubuntu distribution in WSL
   - The Swarmonomicon code at `C:\Users\Dan\lab\madness_interactive\projects\common\Swarmonomicon`

## Available Scripts

We provide three scripts to manage the build and deployment process:

1. **test_wsl.sh**: Tests if your WSL environment is properly set up
2. **build_and_deploy_wsl.sh**: Copies the source code to WSL, builds it, and deploys to EC2
3. **build_direct_wsl.sh**: Builds directly from the existing Windows path, faster if your code is already on Windows

## Testing Your Setup

Before attempting a build, run the test script to verify your environment:

```bash
./test_wsl.sh
```

This script checks:
- SSH connectivity to your Windows machine
- WSL availability and Ubuntu installation
- Rust installation in WSL (will be installed automatically if missing)
- Access to Windows filesystem from WSL
- Existence of the project directory
- SSH connectivity to your EC2 instance

## Building and Deploying

### Option 1: Direct Build (Recommended)

If your code is already in the Windows filesystem, use:

```bash
./build_direct_wsl.sh
```

This script:
- Connects to your Windows machine via SSH
- Runs WSL and navigates to your project directory
- Builds the project with `cargo build --release`
- Copies the binaries to your EC2 instance
- Optionally restarts services on your EC2 instance

### Option 2: Full Copy and Build

If you need to copy the latest code from your Mac to build:

```bash
./build_and_deploy_wsl.sh
```

This script:
- Creates an archive of your current branch
- Copies it to your Windows machine
- Extracts and builds it in WSL
- Transfers the binaries to your EC2 instance
- Optionally restarts services on your EC2 instance

## How It Works

The cross-compilation process works as follows:

1. Connect to Windows machine via SSH
2. Launch WSL from Windows
3. Build the Rust project in the Linux environment
4. Extract the compiled binaries
5. Transfer to EC2 instance (either directly from WSL or via your Mac)

## Troubleshooting

### SSH Connection Issues

If you encounter SSH connectivity problems:

1. Test basic SSH connectivity:
   ```bash
   ssh salad
   ssh eaws
   ```

2. Verify your SSH config in `~/.ssh/config`:
   ```
   Host salad
       HostName your-windows-machine-ip-or-hostname
       User your-windows-username
       IdentityFile ~/.ssh/your-windows-key

   Host eaws
       HostName your-ec2-instance-ip-or-dns
       User ubuntu
       IdentityFile ~/.ssh/your-ec2-key.pem
   ```

### WSL Issues

If WSL is not working properly:

1. On your Windows machine, check WSL status:
   ```
   wsl --status
   wsl -l -v
   ```

2. Make sure Ubuntu is installed:
   ```
   wsl --install -d Ubuntu
   ```

### Build Failures

If the build fails:

1. Check if Rust is installed in WSL:
   ```
   rustc --version
   ```

2. Try installing Rust manually in WSL:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Check for dependency issues:
   ```
   sudo apt update
   sudo apt install pkg-config libssl-dev libcurl4-openssl-dev cmake gcc g++ libc6-dev git
   ```

## Next Steps

After deployment, you can check the status of your services on the EC2 instance:

```bash
ssh eaws "cd ~/Swarmonomicon/bin && ./restart_services.sh"
``` 
