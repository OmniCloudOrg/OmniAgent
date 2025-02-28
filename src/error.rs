use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OmniAgentError {
    #[error("Docker error: {0}")]
    DockerError(#[from] bollard::errors::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Docker not installed")]
    DockerNotInstalled,
    
    #[error("Docker not running")]
    DockerNotRunning,
    
    #[error("Docker initialization failed: {0}")]
    DockerInitFailed(String),
    
    #[error("Docker command execution failed: {0}")]
    CommandExecutionFailed(String),
    
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
    
    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Rocket server error")]
    RocketError,
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type OmniAgentResult<T> = Result<T, OmniAgentError>;