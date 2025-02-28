use bollard::Docker;
use bollard::system::Version;
use futures::TryStreamExt;
use std::process::Command;
use tracing::{debug, error, info, warn};
use which::which;

use crate::error::{OmniAgentError, OmniAgentResult};

pub struct DockerManager {
    client: Docker,
}

impl DockerManager {
    /// Create a new Docker manager
    pub async fn new() -> OmniAgentResult<Self> {
        // Check if Docker is installed
        if which("docker").is_err() {
            error!("Docker is not installed on this system");
            return Err(OmniAgentError::DockerNotInstalled);
        }

        // Attempt to connect to Docker
        let client = match Docker::connect_with_local_defaults() {
            Ok(client) => {
                // Check if we can communicate with Docker
                match client.version().await {
                    Ok(version) => {
                        info!(
                            "Connected to Docker {} (API v{})",
                            version.version.unwrap_or_default(),
                            version.api_version.unwrap_or_default()
                        );
                        client
                    }
                    Err(e) => {
                        error!("Docker is installed but not running: {}", e);
                        return Err(OmniAgentError::DockerNotRunning);
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to Docker: {}", e);
                return Err(OmniAgentError::DockerError(e));
            }
        };

        Ok(DockerManager { client })
    }

    /// Initialize Docker if not already running
    pub async fn initialize() -> OmniAgentResult<Self> {
        info!("Checking for Docker installation");
        let docker_installed = which("docker").is_ok();

        if !docker_installed {
            #[cfg(target_os = "linux")]
            {
                info!("Attempting to install Docker on Linux");
                // This is a simplified example. In a real application,
                // you would want more robust installation logic.
                let output = Command::new("sh")
                    .arg("-c")
                    .arg("curl -fsSL https://get.docker.com | sh")
                    .output()?;

                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr);
                    error!("Failed to install Docker: {}", error);
                    return Err(OmniAgentError::DockerInitFailed(format!(
                        "Installation failed: {}",
                        error
                    )));
                }
                info!("Docker installed successfully");
            }

            #[cfg(target_os = "macos")]
            {
                error!("Docker needs to be installed manually on macOS");
                info!("Please download Docker Desktop from https://www.docker.com/products/docker-desktop");
                return Err(OmniAgentError::DockerInitFailed(
                    "Please install Docker Desktop for Mac manually".to_string(),
                ));
            }

            #[cfg(target_os = "windows")]
            {
                error!("Docker needs to be installed manually on Windows");
                info!("Please download Docker Desktop from https://www.docker.com/products/docker-desktop");
                return Err(OmniAgentError::DockerInitFailed(
                    "Please install Docker Desktop for Windows manually".to_string(),
                ));
            }
        }

        // Start Docker service if not running
        info!("Attempting to start Docker service");
        Self::start_docker_service()?;

        // Try to connect again
        Self::new().await
    }

    /// Start the Docker service based on the platform
    fn start_docker_service() -> OmniAgentResult<()> {
        #[cfg(target_os = "linux")]
        {
            info!("Starting Docker service on Linux");
            let output = Command::new("sh")
                .arg("-c")
                .arg("sudo systemctl start docker")
                .output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                warn!("Failed to start Docker service: {}", error);
                return Err(OmniAgentError::DockerInitFailed(format!(
                    "Service start failed: {}",
                    error
                )));
            }
        }

        #[cfg(target_os = "macos")]
        {
            info!("Starting Docker service on macOS");
            let output = Command::new("open")
                .arg("-a")
                .arg("Docker")
                .output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                warn!("Failed to start Docker service: {}", error);
                return Err(OmniAgentError::DockerInitFailed(format!(
                    "Service start failed: {}",
                    error
                )));
            }
        }

        #[cfg(target_os = "windows")]
        {
            info!("Starting Docker service on Windows");
            let output = Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg("")
                .arg("\"Docker Desktop\"")
                .output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                warn!("Failed to start Docker service: {}", error);
                return Err(OmniAgentError::DockerInitFailed(format!(
                    "Service start failed: {}",
                    error
                )));
            }
        }

        // Give Docker some time to start
        std::thread::sleep(std::time::Duration::from_secs(5));
        Ok(())
    }

    /// Get Docker version information
    pub async fn get_version(&self) -> OmniAgentResult<Version> {
        debug!("Getting Docker version");
        match self.client.version().await {
            Ok(version) => {
                info!("Docker version: {:?}", version);
                Ok(version)
            }
            Err(e) => {
                error!("Failed to get Docker version: {}", e);
                Err(OmniAgentError::DockerError(e))
            }
        }
    }
}