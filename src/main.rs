use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::collections::HashMap;
use chrono::prelude::*;

mod metrics;
use metrics::MetricsClient;

//-----------------------------------------------------------------------------
// Data structures
//-----------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct Container {
    id: String,
    name: String,
    status: String,
    image: String,
    created: String,
    ports: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ContainerConfig {
    name: String,
    image: String,
    ports: Vec<String>,
    environment: Option<HashMap<String, String>>,
}

//-----------------------------------------------------------------------------
// Error handling
//-----------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

//-----------------------------------------------------------------------------
// Container management functions
//-----------------------------------------------------------------------------

async fn list_containers() -> impl Responder {
    let output = Command::new("docker")
        .args(["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Image}}\t{{.CreatedAt}}\t{{.Ports}}"])
        .output();

    match output {
        Ok(output) => {
            let containers = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.split('\t').collect();
                    Container {
                        id: parts.get(0).unwrap_or(&"").to_string(),
                        name: parts.get(1).unwrap_or(&"").to_string(),
                        status: parts.get(2).unwrap_or(&"").to_string(),
                        image: parts.get(3).unwrap_or(&"").to_string(),
                        created: parts.get(4).unwrap_or(&"").to_string(),
                        ports: parts.get(5).unwrap_or(&"").split(',').map(String::from).collect(),
                    }
                })
                .collect::<Vec<Container>>();

            HttpResponse::Ok().json(containers)
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to list containers: {}", e),
            })
        }
    }
}

async fn create_container(config: web::Json<ContainerConfig>) -> impl Responder {
    let mut cmd = Command::new("docker");
    cmd.arg("run")
        .arg("-d")
        .arg("--name")
        .arg(&config.name);

    // Add port mappings
    for port in &config.ports {
        cmd.arg("-p").arg(port);
    }

    // Add environment variables
    if let Some(env) = &config.environment {
        for (key, value) in env {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }
    }

    cmd.arg(&config.image);

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                HttpResponse::Ok().json(Container {
                    id: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    name: config.name.clone(),
                    status: "created".to_string(),
                    image: config.image.clone(),
                    created: chrono::Local::now().to_rfc3339(),
                    ports: config.ports.clone(),
                })
            } else {
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: String::from_utf8_lossy(&output.stderr).to_string(),
                })
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to create container: {}", e),
            })
        }
    }
}

async fn stop_container(container_id: web::Path<String>) -> impl Responder {
    let output = Command::new("docker")
        .args(["stop", &container_id])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                HttpResponse::Ok().json(format!("Container {} stopped successfully", container_id))
            } else {
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: String::from_utf8_lossy(&output.stderr).to_string(),
                })
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to stop container: {}", e),
            })
        }
    }
}

async fn start_container(container_id: web::Path<String>) -> impl Responder {
    let output = Command::new("docker")
        .args(["start", &container_id])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                HttpResponse::Ok().json(format!("Container {} started successfully", container_id))
            } else {
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: String::from_utf8_lossy(&output.stderr).to_string(),
                })
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to start container: {}", e),
            })
        }
    }
}

async fn remove_container(container_id: web::Path<String>) -> impl Responder {
    let output = Command::new("docker")
        .args(["rm", "-f", &container_id])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                HttpResponse::Ok().json(format!("Container {} removed successfully", container_id))
            } else {
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: String::from_utf8_lossy(&output.stderr).to_string(),
                })
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to remove container: {}", e),
            })
        }
    }
}

async fn get_container_logs(container_id: web::Path<String>) -> impl Responder {
    let output = Command::new("docker")
        .args(["logs", "--tail", "100", &container_id])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                HttpResponse::Ok().body(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: String::from_utf8_lossy(&output.stderr).to_string(),
                })
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get container logs: {}", e),
            })
        }
    }
}

//-----------------------------------------------------------------------------
// Router and server initialization
//-----------------------------------------------------------------------------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting OmniAgent server on http://127.0.0.1:8080");

    // Initialize metrics client in a separate task
    tokio::spawn(async move {
        loop {
            match MetricsClient::connect("ws://metrics-server:8081/ws").await {
                Ok(mut client) => {
                    println!("Connected to metrics server");
                    if let Err(e) = client.start_metrics_stream().await {
                        eprintln!("Metrics client error: {:?}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to metrics server: {:?}", e);
                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    // Your existing HTTP server setup
    HttpServer::new(|| {
        App::new()
            .route("/containers", web::get().to(list_containers))
            .route("/containers", web::post().to(create_container))
            .route("/containers/{id}/start", web::post().to(start_container))
            .route("/containers/{id}/stop", web::post().to(stop_container))
            .route("/containers/{id}", web::delete().to(remove_container))
            .route("/containers/{id}/logs", web::get().to(get_container_logs))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}