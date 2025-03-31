# OmniAgent

![OmniAgent Logo](https://placeholder.pics/svg/300x100/DEDEDE/555555/OmniAgent)

[![Docker](https://github.com/OmniCloudOrg/OmniAgent/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/OmniCloudOrg/OmniAgent/actions/workflows/docker-publish.yml)
[![Rust](https://github.com/OmniCloudOrg/OmniAgent/actions/workflows/rust.yml/badge.svg)](https://github.com/OmniCloudOrg/OmniAgent/actions/workflows/rust.yml)
[![Release](https://github.com/OmniCloudOrg/OmniAgent/actions/workflows/release.yml/badge.svg)](https://github.com/OmniCloudOrg/OmniAgent/actions/workflows/release.yml)

OmniAgent is a robust, cross-platform container management and deployment agent built in Rust. It provides a RESTful API interface to Docker operations, making it easier to manage containerized applications across distributed environments.

## ✨ Features

- 🐳 **Complete Docker Management**: Control containers, images, volumes, and networks through a simple API
- 📊 **Real-time Metrics**: Prometheus-compatible metrics for monitoring system and container performance
- 🔄 **Auto-Discovery**: Self-initializes Docker if not already running (platform aware)
- 🔒 **Secure API**: Built-in token-based authentication system
- 🌐 **Cross-Platform**: Runs on Linux, macOS, and Windows
- 🚀 **Efficient**: Small memory footprint and fast performance with Rust
- 📡 **Heartbeat System**: Built-in communication with centralized director services
- 🔌 **Extensible**: Modular design makes it easy to add new features

## 🚀 Quick Start

### Using Docker (Recommended)

```bash
docker run -d --name omniagent \
  -p 8081:8081 \
  -p 2375:2375 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/yourusername/omniagent:latest
```

### From Binary

1. Download the latest binary for your platform from the [Releases](https://github.com/OmniCloudOrg/OmniAgent/releases) page.
2. Make it executable (Linux/macOS): `chmod +x omni-agent`
3. Run it: `./omni-agent`

### Building from Source

Prerequisites: Rust and Cargo installed

```bash
# Clone the repository
git clone https://github.com/yourusername/OmniAgent.git
cd OmniAgent

# Build the project
cargo build --release

# Run the agent
./target/release/omni-agent
```

## 🔌 API Endpoints

OmniAgent exposes a RESTful API on port 8081. Here are the core endpoints:

### Container Management
- `GET /api/containers` - List all containers
- `GET /api/containers/{id}` - Get container details
- `POST /api/containers` - Create a new container
- `POST /api/containers/{id}/start` - Start a container
- `POST /api/containers/{id}/stop` - Stop a container
- `POST /api/containers/{id}/restart` - Restart a container
- `DELETE /api/containers/{id}` - Remove a container
- `GET /api/containers/{id}/logs` - Get container logs
- `POST /api/containers/{id}/exec` - Execute a command in a container

### Image Management
- `GET /api/images` - List all images
- `POST /api/images/pull` - Pull an image
- `DELETE /api/images/{id}` - Remove an image
- `POST /api/images/build` - Build an image
- `GET /api/images/{id}` - Get image details

### Volume Management
- `GET /api/volumes` - List all volumes
- `POST /api/volumes` - Create a volume
- `GET /api/volumes/{name}` - Get volume details
- `DELETE /api/volumes/{name}` - Remove a volume
- `POST /api/volumes/prune` - Prune unused volumes

### Network Management
- `GET /api/networks` - List all networks
- `POST /api/networks` - Create a network
- `GET /api/networks/{id}` - Get network details
- `DELETE /api/networks/{id}` - Remove a network
- `POST /api/networks/{id}/connect` - Connect a container to a network
- `POST /api/networks/{id}/disconnect` - Disconnect a container from a network
- `POST /api/networks/prune` - Prune unused networks

### System Management
- `GET /api/system/health` - Get system health
- `GET /api/system/metrics` - Get system metrics
- `GET /api/system/events` - Get system events
- `POST /api/system/prune` - Prune unused Docker resources

### Agent Management
- `GET /api/agent/status` - Get agent status
- `POST /api/agent/register` - Register agent with a director
- `POST /api/agent/update` - Update agent configuration

## 📊 Metrics

Metrics are exposed in Prometheus format at `/metrics` and in JSON format at `/metrics/json`.

## 🏗️ Architecture

OmniAgent follows a modular architecture:

- **Docker Manager**: Core component for interacting with Docker
- **API Layer**: Provides RESTful endpoints
- **Authentication**: Token-based authentication system
- **Metrics Collector**: Gathers system and container metrics
- **Models**: Data structures for API communication

## 🖥️ Cross-Platform Support

OmniAgent supports:

- Linux (x86_64, ARM64, ARMv7)
- macOS (x86_64, ARM64)
- Windows (x86_64, i686, ARM64)

Each platform has platform-specific optimizations for Docker communication.

## 🔄 Using with Orchestration Systems

OmniAgent is designed to be packed into the VM images OmniDirectory deploys. This allows the platform to manage apps easily within a given worker.

## 📚 Advanced Usage

### Using with a Director Service

OmniAgent can be registered with a central director service for fleet management:

```bash
curl -X POST http://localhost:8081/api/agent/register \
  -H "Content-Type: application/json" \
  -d '{"director_url": "http://director.example.com", "token": "your-token"}'
```

### Custom Container Configurations

Create complex container configurations:

```bash
curl -X POST http://localhost:8081/api/containers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-app",
    "image": "nginx:latest",
    "ports": [{"host": 8080, "container": 80, "protocol": "tcp"}],
    "environment": {"DEBUG": "true", "NODE_ENV": "production"},
    "volumes": [{"host": "/data", "container": "/app/data", "read_only": false}],
    "restart_policy": "always"
  }'
```

## 🛠️ Development

### Running Tests

```bash
cargo test
```

### Building for Different Platforms

```bash
cargo build --target x86_64-unknown-linux-gnu
cargo build --target aarch64-apple-darwin
cargo build --target x86_64-pc-windows-msvc
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 👥 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📞 Contact

- Project Link: [https://github.com/OmniCloudOrg/OmniAgent](https://github.com/OmniCloudOrg/OmniAgent)
- Issue Tracker: [https://github.com/OmniCloudOrg/OmniAgent/issues](https://github.com/OmniCloudOrg/OmniAgent/issues)

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/tristanpoland">Tristan J. Poland</a> and the OmniCloud Community
</p>