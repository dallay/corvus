FROM ubuntu:22.04

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install common development tools and runtimes
# - Node.js: Install v20 (LTS) from NodeSource
# - Core: curl, git, vim, build-essential (gcc, make)
# - Python: python3, pip
# - Network: ping, dnsutils
RUN apt-get update && apt-get install -y curl && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y \
    build-essential \
    dnsutils \
    git \
    iputils-ping \
    nano \
    net-tools \
    nodejs \
    python3 \
    python3-pip \
    sudo \
    unzip \
    vim \
    wget \
    zip \
    && useradd -m -s /bin/bash -u 1000 developer && \
    echo "developer ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/developer && \
    chmod 0440 /etc/sudoers.d/developer && \
    rm -rf /var/lib/apt/lists/* && \
    node --version && npm --version

# Set up the workspace
USER developer
WORKDIR /home/developer/workspace

# Default command
CMD ["/bin/bash"]
