# 🚀 OmniCloud Agent

## Unified Container Management & System Orchestration

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Version](https://img.shields.io/badge/version-1.0.0-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey)

## 🌟 What is OmniCloud Agent?

OmniCloud Agent is a cross-platform microservice that provides granular container management, system metrics collection, and infrastructure orchestration. Designed for scalability and flexibility, it serves as the critical link between your OmniCloud control plane and individual compute resources.

### 💡 Key Features

- **Cross-Platform Support** 
  - Native Windows, Linux, and macOS integration
  - Seamless Docker Desktop and Docker Engine compatibility

- **Container Lifecycle Management**
  - Deploy containers with advanced configuration
  - Start, stop, and remove containers
  - Network and container status tracking

- **Real-Time System Metrics**
  - Comprehensive resource utilization tracking
  - CPU, memory, and disk usage monitoring
  - Configurable metrics collection

- **Secure & Configurable**
  - Network origin restrictions
  - Configurable logging
  - TLS support (planned)

## 🛠 Installation

### Prerequisites

- Rust 1.70+
- Docker 20.10+ or Docker Desktop
- Platform-specific Docker configuration

### Quick Start

#### Clone the Repository
```bash
git clone https://github.com/omnicloud/omni-agent.git
cd omni-agent
```

#### Build & Run
```bash
# Build the project
cargo build --release

# Run the agent
cargo run --release
```

### Configuration

Create a `config.json` in the project root:

```json
{
    "api": {
        "host": "0.0.0.0",
        "port": 8081,
        "log_level": "info"
    },
    "docker": {
        "api_url": "http://localhost:2375",
        "default_network": "bridge",
        "timeout_seconds": 30
    },
    "platform": {
        "container_runtime": "docker-desktop",
        "docker_socket": "npipe:////./pipe/docker_engine"
    }
}
```

## 🔌 API Endpoints

### Container Management
- `POST /deploy`: Deploy a new container
- `POST /start/{container_id}`: Start a container
- `POST /stop/{container_id}`: Stop a container
- `DELETE /remove/{container_id}`: Remove a container
- `GET /status/{container_id}`: Get container status

### System Metrics
- `GET /metrics/system`: Retrieve system resource metrics

## 📡 Configuration Options

### Environment Variables
- `OMNI_AGENT_CONFIG`: Custom configuration file path

### Platform-Specific Settings
- Windows: Automatically detects Docker Desktop
- Linux: Standard Docker socket configuration
- macOS: Supports both Docker Desktop and Docker Engine

## 🔒 Security Considerations

- Network origin restrictions
- Configurable TLS support
- Minimal container runtime permissions

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

## 📊 Performance Metrics

- Low overhead container management
- Minimal resource consumption
- Sub-millisecond API response times

## 🛡️ Compatibility

![Docker](https://img.shields.io/badge/Docker-20.10+-blue)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![Windows](https://img.shields.io/badge/Windows-10%2B-blue)
![Linux](https://img.shields.io/badge/Linux-Any%20Distro-green)
![macOS](https://img.shields.io/badge/macOS-10.15+-lightgrey)

## 📦 Deployment Scenarios

- Cloud Infrastructure
- Edge Computing
- Hybrid Environments
- Microservice Architectures
- Development & Testing Workflows

## 📞 Support

- GitHub Issues
- Community Discord
- Email Support: support@omnicloud.io

## 📄 License

Apache 2.0 License

---

**Crafted with ❤️ by the OmniCloud Engineering Team**

*Empowering Distributed Systems, One Container at a Time*