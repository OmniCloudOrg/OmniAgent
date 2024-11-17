use tokio::{
    time::{self, Duration},
    net::TcpStream,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message,
    WebSocketStream,
    MaybeTlsStream,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::process::Command;
use url::Url;
use std::fmt;

// Custom error type that implements Send
#[derive(Debug)]
pub enum MetricsError {
    WebSocket(tokio_tungstenite::tungstenite::Error),
    Io(std::io::Error),
    Parse(String),
    Url(url::ParseError),
    Serialize(serde_json::Error),
}

impl std::error::Error for MetricsError {}

impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsError::WebSocket(e) => write!(f, "WebSocket error: {}", e),
            MetricsError::Io(e) => write!(f, "IO error: {}", e),
            MetricsError::Parse(e) => write!(f, "Parse error: {}", e),
            MetricsError::Url(e) => write!(f, "URL parse error: {}", e),
            MetricsError::Serialize(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for MetricsError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        MetricsError::WebSocket(err)
    }
}

impl From<std::io::Error> for MetricsError {
    fn from(err: std::io::Error) -> Self {
        MetricsError::Io(err)
    }
}

impl From<url::ParseError> for MetricsError {
    fn from(err: url::ParseError) -> Self {
        MetricsError::Url(err)
    }
}

impl From<serde_json::Error> for MetricsError {
    fn from(err: serde_json::Error) -> Self {
        MetricsError::Serialize(err)
    }
}

// Metrics data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerMetrics {
    container_id: String,
    name: String,
    timestamp: i64,
    cpu_usage: f64,
    memory_usage: u64,
    memory_limit: u64,
    network_rx_bytes: u64,
    network_tx_bytes: u64,
}

pub struct MetricsClient {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl MetricsClient {
    pub async fn connect(url: &str) -> Result<Self, MetricsError> {
        let url = Url::parse(url)?;
        let (ws_stream, _) = connect_async(url.as_str()).await?;
        
        Ok(MetricsClient {
            ws_stream,
        })
    }

    async fn collect_container_metrics(container_id: &str) -> Result<ContainerMetrics, MetricsError> {
        let output = Command::new("docker")
            .args([
                "stats",
                container_id,
                "--no-stream",
                "--format",
                "{{.Container}}\t{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}",
            ])
            .output()?;

        let stats = String::from_utf8(output.stdout)
            .map_err(|e| MetricsError::Parse(e.to_string()))?;
        let parts: Vec<&str> = stats.trim().split('\t').collect();
        
        if parts.len() < 5 {
            return Err(MetricsError::Parse("Invalid stats format".into()));
        }

        // Parse memory usage
        let mem_parts: Vec<&str> = parts[3].split('/').collect();
        let memory_usage = parse_bytes(mem_parts[0])?;
        let memory_limit = parse_bytes(mem_parts[1])?;

        // Parse network I/O
        let net_parts: Vec<&str> = parts[4].split('/').collect();
        let network_rx_bytes = parse_bytes(net_parts[0])?;
        let network_tx_bytes = parse_bytes(net_parts[1])?;

        // Parse CPU percentage
        let cpu_usage = parts[2]
            .trim_end_matches('%')
            .parse::<f64>()
            .map_err(|e| MetricsError::Parse(e.to_string()))?;

        Ok(ContainerMetrics {
            container_id: parts[0].to_string(),
            name: parts[1].to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            cpu_usage,
            memory_usage,
            memory_limit,
            network_rx_bytes,
            network_tx_bytes,
        })
    }

    pub async fn start_metrics_stream(&mut self) -> Result<(), MetricsError> {
        let mut interval = time::interval(Duration::from_secs(10));

        loop {
            interval.tick().await;

            let output = Command::new("docker")
                .args(["ps", "-q"])
                .output()?;

            let containers = String::from_utf8(output.stdout)
                .map_err(|e| MetricsError::Parse(e.to_string()))?;

            for container_id in containers.lines() {
                match Self::collect_container_metrics(container_id).await {
                    Ok(metrics) => {
                        let message = serde_json::to_string(&metrics)?;
                        self.ws_stream.send(Message::Text(message)).await?;
                    }
                    Err(e) => eprintln!("Error collecting metrics for container {}: {}", container_id, e),
                }
            }
        }
    }
}

fn parse_bytes(s: &str) -> Result<u64, MetricsError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(0);
    }

    let parts: Vec<&str> = s.split_inclusive(char::is_alphabetic).collect();
    if parts.len() != 2 {
        return Err(MetricsError::Parse("Invalid byte format".into()));
    }

    let number = parts[0].trim()
        .parse::<f64>()
        .map_err(|e| MetricsError::Parse(e.to_string()))?;
    let unit = parts[1].trim().to_uppercase();

    let multiplier = match unit.as_str() {
        "B" => 1.0,
        "KB" | "KIB" => 1024.0,
        "MB" | "MIB" => 1024.0 * 1024.0,
        "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        _ => return Err(MetricsError::Parse("Unknown unit".into())),
    };

    Ok((number * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bytes() {
        assert_eq!(parse_bytes("1.5KiB").unwrap(), 1536);
        assert_eq!(parse_bytes("2.5MiB").unwrap(), 2621440);
        assert_eq!(parse_bytes("1GiB").unwrap(), 1073741824);
        assert!(parse_bytes("invalid").is_err());
    }
}