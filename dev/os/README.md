# Corvus Agent Testing Environments

This directory contains Docker images for testing Corvus agent installation scripts and validating agent functionality in controlled environments.

## Purpose

These Dockerfiles create isolated, reproducible testing environments that simulate different Linux distributions. They are designed for:

- **Installation Script Testing**: Validate the Corvus installation script works across different Linux distributions
- **Agent Functionality Testing**: Test all Corvus agent features (tools, channels, providers, memory backends)
- **CI/CD Integration**: Use in automated testing pipelines
- **Development Verification**: Ensure Corvus works correctly in various OS environments

## Available Images

### Ubuntu (Dockerfile.ubuntu)

Based on `ubuntu:24.04` with full-featured packages. Ideal for testing on Debian-based systems.

**Build:**
```bash
docker build -f dev/os/Dockerfile.ubuntu -t corvus-test:ubuntu .
```

**Run:**
```bash
docker run -it corvus-test:ubuntu bash
```

### Alpine (Dockerfile.alpine)

Based on `alpine:3.20` with minimal packages. Perfect for testing on lightweight/embedded systems.

**Build:**
```bash
docker build -f dev/os/Dockerfile.alpine -t corvus-test:alpine .
```

**Run:**
```bash
docker run -it corvus-test:alpine sh
```

## Included Tools

Both images include the following tools:

| Category | Tools |
|----------|-------|
| **Version Control** | `git` |
| **Networking** | `curl`, `wget`, `iputils-ping`, `dnsutils`, `net-tools` |
| **Editors** | `vim`, `nano` |
| **Build** | `build-essential` / `build-base`, `pkg-config` |
| **Security** | `ca-certificates`, `openssl` |
| **Compression** | `tar`, `gzip`, `bzip2`, `xz-utils`, `zip`, `unzip` |
| **System** | `procps`, `htop`, `tree` |
| **Text Processing** | `grep`, `sed`, `gawk`, `findutils` |
| **Runtimes** | Rust (stable), Node.js 20 LTS, Python 3 |

## User Setup

Both images create a non-root user `tester` with sudo privileges:

| Image | UID | Username | Workspace |
|-------|-----|----------|-----------|
| Ubuntu | 1001 | `tester` | `/home/tester/workspace` |
| Alpine | 1000 | `tester` | `/home/tester/workspace` |

- **Password**: none (passwordless sudo enabled)

This setup simulates a typical development environment while maintaining security best practices.

## Testing Corvus Installation

### Option 1: Using npm/pnpm

```bash
# Inside the container
npm install -g @dallay/corvus
# or
pnpm add -g @dallay/corvus

corvus --help
corvus status
```

### Option 2: Building from source

```bash
# Inside the container
git clone https://github.com/dallay/corvus.git
cd corvus
cargo build --release
cargo install --path .
```

### Option 3: Using npx

```bash
# Quick test without installation
npx @dallay/corvus --help
```

## Environment Variables

Both images set the following environment variables:

| Variable | Value |
|----------|-------|
| `HOME` | `/home/tester` |
| `USER` | `tester` |
| `PATH` | Includes Rust/Cargo binaries |

For Alpine, additional variables:
| Variable | Value |
|----------|-------|
| `RUSTUP_HOME` | `/root/.cargo` |
| `CARGO_HOME` | `/root/.cargo` |

## Security Considerations

- Images run as non-root user by default
- Passwordless sudo is configured for testing convenience
- Use `--read-only` flag in production CI to prevent modifications
- Always review and restrict capabilities in production environments

## Advanced Usage

### Using Docker Compose

```yaml
# docker-compose.test.yml
services:
  ubuntu-test:
    build:
      context: .
      dockerfile: dev/os/Dockerfile.ubuntu
    volumes:
      - ./test-scripts:/home/tester/test-scripts
    working_dir: /home/tester/test-scripts

  alpine-test:
    build:
      context: .
      dockerfile: dev/os/Dockerfile.alpine
    volumes:
      - ./test-scripts:/home/tester/test-scripts
    working_dir: /home/tester/test-scripts
```

```bash
docker-compose -f docker-compose.test.yml run ubuntu-test
```

### Running Tests Automatically

```bash
# Build both images
docker build -f dev/os/Dockerfile.ubuntu -t corvus-test:ubuntu .
docker build -f dev/os/Dockerfile.alpine -t corvus-test:alpine .

# Run installation test script
docker run --rm corvus-test:ubuntu bash -c "
    corvus --version && echo 'Ubuntu: SUCCESS'
"

docker run --rm corvus-test:alpine sh -c "
    corvus --version && echo 'Alpine: SUCCESS'
"
```

## Troubleshooting

### Network Issues

If you encounter network problems, verify DNS resolution:
```bash
docker run --rm corvus-test:ubuntu ping -c 3 google.com
```

### Rust Installation Fails

The Rust installer requires network access. If it fails:
```bash
# In container - manual installation
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Permission Denied Errors

Ensure you're running as the `tester` user:
```bash
docker run -it --user tester corvus-test:ubuntu bash
```

## Contributing

When adding new tools or modifying these Dockerfiles:

1. Update this README with any new dependencies
2. Test both images build successfully
3. Verify the installation script works in both environments
4. Consider Alpine-specific package names (they differ from Debian)

## References

- [Corvus Agent Runtime](https://github.com/dallay/corvus)
- [Docker Multi-stage Builds](https://docs.docker.com/develop/develop-images/multistage-build/)
- [Alpine Linux](https://alpinelinux.org/)
- [Ubuntu Packages](https://packages.ubuntu.com/)
