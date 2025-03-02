use rocket::{get, post, delete, routes, serde::json::Json, http::Status, Config as RocketConfig};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::collections::HashMap;
use std::fs;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use sysinfo::{System};
use console::{style, Term};
use serde_json;

// Add Windows-specific imports
#[cfg(windows)]
use std::os::windows::process::CommandExt;

lazy_static! {
    static ref DOCKER_AVAILABLE: AtomicBool = AtomicBool::new(false);
}

// Configuration struct that matches the JSON structure
#[derive(Debug, Deserialize, Clone)]
struct OmniAgentConfig {
    api: ApiConfig,
    docker: DockerConfig,
    system_metrics: SystemMetricsConfig,
    security: SecurityConfig,
    logging: LoggingConfig,
    platform: PlatformConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct PlatformConfig {
    #[serde(default = "default_container_runtime")]
    container_runtime: String,
    #[serde(default = "default_docker_socket")]
    docker_socket: String,
}

fn default_container_runtime() -> String {
    if cfg!(windows) {
        "docker-desktop".to_string()
    } else {
        "docker".to_string()
    }
}

fn default_docker_socket() -> String {
    if cfg!(windows) {
        "npipe:////./pipe/docker_engine".to_string()
    } else {
        "unix:///var/run/docker.sock".to_string()
    }
}

#[derive(Debug, Deserialize, Clone)]
struct ApiConfig {
    host: String,
    port: u16,
    log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
struct DockerConfig {
    api_url: String,
    default_network: String,
    timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
struct SystemMetricsConfig {
    refresh_interval_ms: u64,
    log_metrics: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct SecurityConfig {
    allowed_networks: Vec<String>,
    enable_tls: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct LoggingConfig {
    format: String,
    output: String,
}

// Check Docker availability on startup
fn check_docker_availability() -> bool {
    Command::new("docker")
        .arg("info")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// Load configuration from a JSON file
fn load_config() -> Result<OmniAgentConfig, Box<dyn std::error::Error>> {
    // First, check for a config file path from an environment variable
    let config_path = env::var("OMNI_AGENT_CONFIG")
        .unwrap_or_else(|_| String::from("config.json"));

    // Read the configuration file
    let config_contents = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| {
            // If no config file is found, return a default configuration
            r#"{
                "api": {"host": "0.0.0.0", "port": 8081, "log_level": "info"},
                "docker": {"api_url": "http://localhost:2375", "default_network": "bridge", "timeout_seconds": 30},
                "system_metrics": {"refresh_interval_ms": 5000, "log_metrics": false},
                "security": {"allowed_networks": ["localhost", "127.0.0.1"], "enable_tls": false},
                "logging": {"format": "json", "output": "stdout"},
                "platform": {
                    "container_runtime": "",
                    "docker_socket": ""
                }
            }"#.to_string()
        });

    // Parse the JSON configuration
    let config: OmniAgentConfig = serde_json::from_str(&config_contents)?;
    Ok(config)
}

const BANNER: &str = r#"
   ____  __  __ _   _ _____             _____  ______ _   _ _______ 
  / __ \|  \/  | \ | |_   _|      /\   / ____||  ____| \ | |__   __|
 | |  | | \  / |  \| | | |       /  \ | |  __|| |__  |  \| |  | |   
 | |  | | |\/| | . ` | | |      / /\ \| | |_ ||  __| | . ` |  | |   
 | |__| | |  | | |\  |_| |_    / ____ \ |__| || |____| |\  |  | |   
  \____/|_|  |_|_| \_|_____|  /_/    \_\_____||______|_| \_|  |_|   v{}"#;

#[macro_use] extern crate rocket;

#[derive(Serialize, Deserialize)]
struct ContainerConfig {
    image: String,
    name: Option<String>,
    env: Option<Vec<String>>,
    ports: Option<HashMap<String, String>>, // "8080/tcp" -> "80"
    volumes: Option<Vec<String>>, // "/host/path:/container/path"
    network: Option<String>,
}

// Windows-specific Docker runtime configuration
fn configure_windows_docker() -> Result<(), String> {
    #[cfg(windows)]
    {
        // Check if Docker Desktop is running
        let output = Command::new("tasklist")
            .arg("/FI")
            .arg("IMAGENAME eq Docker Desktop.exe")
            .output()
            .map_err(|e| format!("Failed to check Docker Desktop: {}", e))?;

        if !output.status.success() {
            return Err("Docker Desktop is not running".to_string());
        }
    }

    Ok(())
}

#[get("/containers")]
fn list_containers() -> Result<Json<Vec<HashMap<String, String>>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    // Use a more verbose command to get full container details
    let output = Command::new("docker")
        .args([
            "ps", 
            "-a", 
            "--format", 
            "{{json .}}"
        ])
        .output()
        .map_err(|_| Status::InternalServerError)?;

    // Capture and log the raw output for debugging
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // If command failed, log the error
    if !output.status.success() {
        eprintln!("Docker PS Command Failed. Stderr: {}", stderr);
        return Err(Status::InternalServerError);
    }

    // Split the output into lines and parse each line
    let containers: Vec<HashMap<String, String>> = stdout
        .lines()
        .filter_map(|line| {
            match serde_json::from_str(line) {
                Ok(container) => Some(container),
                Err(e) => {
                    eprintln!("Failed to parse container JSON: {}", e);
                    None
                }
            }
        })
        .collect();

    // Log the number of containers found
    eprintln!("Found {} containers", containers.len());

    Ok(Json(containers))
}

#[post("/deploy", data = "<config>")]
fn deploy_container(config: Json<ContainerConfig>) -> Result<Json<HashMap<String, String>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    // Construct docker run command using a Vec<String> for flexibility
    let mut docker_cmd = vec![
        "run".to_string(),
        "-d".to_string(), // Run in detached mode
    ];

    // Add name if specified
    if let Some(name) = &config.name {
        docker_cmd.extend([
            "--name".to_string(), 
            name.clone()
        ]);
    }

    // Add environment variables
    if let Some(env) = &config.env {
        docker_cmd.extend(
            env.iter()
                .flat_map(|e| vec!["-e".to_string(), e.clone()])
        );
    }

    // Add volumes
    if let Some(volumes) = &config.volumes {
        docker_cmd.extend(
            volumes.iter()
                .flat_map(|vol| vec!["-v".to_string(), vol.clone()])
        );
    }

    // Add ports with a different approach
    if let Some(ports) = &config.ports {
        docker_cmd.extend(
            ports.iter()
                .map(|(container_port, host_port)| 
                    format!("-p {}:{}", host_port, container_port)
                )
        );
    }

    // Add network if specified
    if let Some(network) = &config.network {
        docker_cmd.extend([
            "--network".to_string(), 
            network.clone()
        ]);
    }

    // Add image
    docker_cmd.push(config.image.clone());

    // Execute docker command
    let output = Command::new("docker")
        .args(&docker_cmd)
        .output()
        .map_err(|_| Status::InternalServerError)?;

    // Process and return response
    let mut response = HashMap::new();
    if output.status.success() {
        response.insert(
            "container_id".to_string(), 
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        );
        response.insert("status".to_string(), "created".to_string());
    } else {
        response.insert(
            "error".to_string(), 
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        );
        response.insert("status".to_string(), "failed".to_string());
    }

    Ok(Json(response))
}

#[post("/start/<container_id>")]
fn start_container(container_id: String) -> Result<Json<HashMap<String, String>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    let output = Command::new("docker")
        .arg("start")
        .arg(container_id)
        .output()
        .map_err(|_| Status::InternalServerError)?;

    let mut response = HashMap::new();
    if output.status.success() {
        response.insert("status".to_string(), "started".to_string());
    } else {
        response.insert(
            "error".to_string(), 
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        );
        response.insert("status".to_string(), "failed".to_string());
    }

    Ok(Json(response))
}

#[post("/stop/<container_id>")]
fn stop_container(container_id: String) -> Result<Json<HashMap<String, String>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    let output = Command::new("docker")
        .arg("stop")
        .arg(container_id)
        .output()
        .map_err(|_| Status::InternalServerError)?;

    let mut response = HashMap::new();
    if output.status.success() {
        response.insert("status".to_string(), "stopped".to_string());
    } else {
        response.insert(
            "error".to_string(), 
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        );
        response.insert("status".to_string(), "failed".to_string());
    }

    Ok(Json(response))
}

#[delete("/remove/<container_id>")]
fn remove_container(container_id: String) -> Result<Json<HashMap<String, String>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    let output = Command::new("docker")
        .arg("rm")
        .arg("-f")
        .arg(container_id)
        .output()
        .map_err(|_| Status::InternalServerError)?;

    let mut response = HashMap::new();
    if output.status.success() {
        response.insert("status".to_string(), "removed".to_string());
    } else {
        response.insert(
            "error".to_string(), 
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        );
        response.insert("status".to_string(), "failed".to_string());
    }

    Ok(Json(response))
}

#[get("/status/<container_id>")]
fn get_container_status(container_id: String) -> Result<Json<HashMap<String, String>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    let output = Command::new("docker")
        .arg("inspect")
        .arg("--format={{json .}}")
        .arg(container_id)
        .output()
        .map_err(|_| Status::InternalServerError)?;

    let status: HashMap<String, String> = 
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .unwrap_or_default();

    Ok(Json(status))
}

#[post("/network/create/<network_name>")]
fn create_network(network_name: String) -> Result<Json<HashMap<String, String>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    let output = Command::new("docker")
        .arg("network")
        .arg("create")
        .arg(network_name)
        .output()
        .map_err(|_| Status::InternalServerError)?;

    let mut response = HashMap::new();
    if output.status.success() {
        response.insert(
            "network_id".to_string(), 
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        );
        response.insert("status".to_string(), "created".to_string());
    } else {
        response.insert(
            "error".to_string(), 
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        );
        response.insert("status".to_string(), "failed".to_string());
    }

    Ok(Json(response))
}

#[get("/networks")]
fn list_networks() -> Result<Json<Vec<HashMap<String, String>>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    let output = Command::new("docker")
        .arg("network")
        .arg("ls")
        .arg("--format")
        .arg("{{json .}}")
        .output()
        .map_err(|_| Status::InternalServerError)?;

    let networks: Vec<HashMap<String, String>> = 
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .unwrap_or_default();

    Ok(Json(networks))
}

#[delete("/network/remove/<network_id>")]
fn remove_network(network_id: String) -> Result<Json<HashMap<String, String>>, Status> {
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        return Err(Status::ServiceUnavailable);
    }

    let output = Command::new("docker")
        .arg("network")
        .arg("rm")
        .arg(network_id)
        .output()
        .map_err(|_| Status::InternalServerError)?;

    let mut response = HashMap::new();
    if output.status.success() {
        response.insert("status".to_string(), "removed".to_string());
    } else {
        response.insert(
            "error".to_string(), 
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        );
        response.insert("status".to_string(), "failed".to_string());
    }

    Ok(Json(response))
}

#[get("/metrics/system")] 
fn stream_system_metrics() -> Json<HashMap<String, String>> {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let mut metrics = HashMap::new();
    
    // Add Docker availability status
    metrics.insert("docker_available".to_string(), DOCKER_AVAILABLE.load(Ordering::Relaxed).to_string());
    
    // Cross-platform CPU usage
    #[cfg(windows)]
    {
        metrics.insert("cpu_usage".to_string(), format!("{:.2}%", 
            sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32
        ));
    }
    
    #[cfg(not(windows))]
    {
        metrics.insert("cpu_usage".to_string(), format!("{:.2}%", sys.global_cpu_info().cpu_usage()));
    }
    
    metrics.insert("memory_usage".to_string(), format!("{:.2} MB", sys.used_memory() as f64 / 1024.0 / 1024.0));
    metrics.insert("total_memory".to_string(), format!("{:.2} MB", sys.total_memory() as f64 / 1024.0 / 1024.0));
    metrics.insert("disk_usage".to_string(), format!("{:.2} MB", sys.used_swap() as f64 / 1024.0 / 1024.0));
    
    Json(metrics)
}

#[launch]
fn rocket() -> _ {
    // Check Docker availability on startup
    DOCKER_AVAILABLE.store(check_docker_availability(), Ordering::Relaxed);

    // If Docker is not available, log a warning
    if !DOCKER_AVAILABLE.load(Ordering::Relaxed) {
        eprintln!("WARNING: Docker is not available. Container operations will be disabled.");
    }

    // Load configuration
    let config = load_config().expect("Failed to load configuration");

    // Windows-specific Docker configuration
    #[cfg(windows)]
    {
        if let Err(e) = configure_windows_docker() {
            eprintln!("Windows Docker Configuration Warning: {}", e);
        }
    }

    // Print banner with version
    println!("{}", style(BANNER.replace("{}", env!("CARGO_PKG_VERSION"))).cyan().bold());
    println!();    

    // Start the API server
    println!();
    println!("{}", style("STARTING API SERVER").yellow().bold());
    
    println!();
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    println!("{} {}", 
        style("▶").green().bold(), 
        style("OMNI AGENT READY").white().bold()
    );
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    println!("  {} {}", style("API:").yellow().bold(), style(format!("http://{}:{}/api", config.api.host, config.api.port)).white());
    println!("  {} {}", style("METRICS:").yellow().bold(), style(format!("http://{}:{}/metrics", config.api.host, config.api.port)).white());
    println!("  {} {}", style("DOCKER TCP:").yellow().bold(), style(&config.docker.api_url).white());
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    
    // Configure Rocket server based on config
    let rocket_config = RocketConfig::figment()
        .merge(("port", config.api.port))
        .merge(("address", config.api.host.clone()));

    rocket::custom(rocket_config).mount("/", routes![
        list_containers,
        deploy_container,
        start_container,
        stop_container,
        remove_container,
        get_container_status,
        create_network,
        list_networks,
        remove_network,
        stream_system_metrics,
    ])
}