FROM ubuntu:24.04

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install common development tools and runtimes
# - Node.js/npm: from Ubuntu's signed package repositories
# - Core: curl, git, vim, build-essential (gcc, make)
# - Python: python3, pip
# - Network: ping, dnsutils
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    dnsutils \
    git \
    iputils-ping \
    nano \
    net-tools \
    nodejs \
    npm \
    python3 \
    python3-pip \
    sudo \
    unzip \
    vim \
    wget \
    zip \
    && if id -u developer >/dev/null 2>&1; then \
         true; \
       elif getent passwd 1000 >/dev/null 2>&1; then \
         useradd -m -s /bin/bash developer; \
       else \
         useradd -m -s /bin/bash -u 1000 developer; \
       fi && \
    echo "developer ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/developer && \
    chmod 0440 /etc/sudoers.d/developer && \
    rm -rf /var/lib/apt/lists/* && \
    node --version && npm --version

# Set up the workspace
USER developer
WORKDIR /home/developer/workspace

# Default command
CMD ["/bin/bash"]
