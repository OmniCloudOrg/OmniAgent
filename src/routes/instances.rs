use rocket::{delete, get, post, patch, put};
use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::State;
use rocket::FromForm;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use bollard::Docker;
use bollard::container::{CreateContainerOptions, Config, StartContainerOptions, StopContainerOptions, RemoveContainerOptions, ListContainersOptions};
use bollard::image::ListImagesOptions;
use bollard::system::EventsOptions;
use futures::stream::StreamExt;

// Data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstance {
    id: String,
    name: String,
    image: String,
    status: String,
    created_at: String,
    ports: Vec<PortMapping>,
    environment: HashMap<String, String>,
    volumes: Vec<VolumeMapping>,
    agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    host_port: u16,
    container_port: u16,
    protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMapping {
    host_path: String,
    container_path: String,
}
#[derive(Debug, Clone, rocket::serde::Serialize, rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AppInstanceRequest {
    name: String,
    image: String,
    ports: Option<Vec<PortMapping>>,
    environment: Option<HashMap<String, String>>,
    volumes: Option<Vec<VolumeMapping>>,
}

// Docker client wrapper
pub struct AppManager {
    docker: Docker,
    instances: Arc<Mutex<HashMap<String, AppInstance>>>,
}

impl AppManager {
    pub fn new() -> Result<Self, String> {
        // Connect to Docker with default configuration
        // Works across platforms without additional config
        let docker = match Docker::connect_with_local_defaults() {
            Ok(docker) => docker,
            Err(e) => return Err(format!("Failed to connect to Docker: {}", e)),
        };
        
        Ok(AppManager {
            docker,
            instances: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

// API Endpoints
#[get("/instances")]
pub async fn list_instances(app_manager: &State<AppManager>) -> Json<Vec<AppInstance>> {
    let mut instances = Vec::new();
    
    // List containers via Docker API
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });
    
    match app_manager.docker.list_containers(options).await {
        Ok(containers) => {
            for container in containers {
                if let (Some(id), Some(image), Some(names), Some(created), Some(status)) = 
                   (container.id, container.image, container.names, container.created, container.status) {
                    if let Some(name) = names.first() {
                        let name = name.trim_start_matches('/').to_string();
                        let app_instance = AppInstance {
                            id: id.clone(),
                            name,
                            image,
                            status,
                            created_at: created.to_string(),
                            ports: Vec::new(), // Would need to parse from container.ports
                            environment: HashMap::new(), // Would need additional API call
                            volumes: Vec::new(), // Would need additional API call
                            agent_id: "current".to_string(), // In a distributed setup, this would be the agent ID
                        };
                        instances.push(app_instance);
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("Failed to list containers: {}", e);
        }
    }
    
    Json(instances)
}

#[get("/instances/<id>")]
pub async fn get_instance(id: String, app_manager: &State<AppManager>) -> Option<Json<AppInstance>> {
    // Get container details via Docker API
    match app_manager.docker.inspect_container(&id, None).await {
        Ok(container) => {
            let config = container.config?;
            let state = container.state?;
            
            let name = container.name?;
            let name = name.trim_start_matches('/').to_string();
            
            let app_instance = AppInstance {
                id: container.id.unwrap_or(id),
                name,
                image: config.image.unwrap_or_default(),
                status: state.status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string()),
                created_at: container.created.unwrap_or_default(),
                ports: Vec::new(), // Would need to parse from container.network_settings
                environment: HashMap::new(), // Would need to parse from config.env
                volumes: Vec::new(), // Would need to parse from container.mounts
                agent_id: "current".to_string(),
            };
            
            Some(Json(app_instance))
        },
        Err(_) => None
    }
}
#[post("/instances", format = "json", data = "<app_req>")]
pub async fn create_instance(app_req: Json<AppInstanceRequest>, app_manager: &State<AppManager>) -> Result<Json<AppInstance>, String> {
    // Prepare container configuration
    let name = app_req.name.clone();
    
    let mut port_bindings = HashMap::new();
    if let Some(ports) = &app_req.ports {
        for port in ports {
            let host_binding = format!("{}:{}", port.host_port, port.container_port);
            port_bindings.insert(
                format!("{}/{}", port.container_port, port.protocol), 
                Some(vec![bollard::models::PortBinding { 
                    host_ip: Some("0.0.0.0".to_string()), 
                    host_port: Some(port.host_port.to_string()) 
                }])
            );
        }
    }
    
    let mut env_vars = Vec::new();
    if let Some(env) = &app_req.environment {
        for (key, value) in env {
            env_vars.push(format!("{}={}", key, value));
        }
    }
    
    let mut volume_bindings = Vec::new();
    if let Some(volumes) = &app_req.volumes {
        for volume in volumes {
            volume_bindings.push(format!("{}:{}", volume.host_path, volume.container_path));
        }
    }
    
    // Create container
    let options = Some(CreateContainerOptions {
        name: &name,
        platform: None,
    });
    
    let config = Config {
        image: Some(app_req.image.clone()),
        env: Some(env_vars),
        exposed_ports: Some(HashMap::new()), // Would need to populate from app_req.ports
        host_config: Some(bollard::models::HostConfig {
            port_bindings: Some(port_bindings),
            binds: Some(volume_bindings),
            ..Default::default()
        }),
        ..Default::default()
    };
    
    match app_manager.docker.create_container(options, config).await {
        Ok(response) => {
            // Start the container
            let id = response.id;
            match app_manager.docker.start_container(&id, None::<StartContainerOptions<String>>).await {
                Ok(_) => {
                    // Create app instance object
                    let app_instance = AppInstance {
                        id: id.clone(),
                        name: app_req.name.clone(),
                        image: app_req.image.clone(),
                        status: "running".to_string(),
                        created_at: chrono::Utc::now().to_string(),
                        ports: app_req.ports.clone().unwrap_or_default(),
                        environment: app_req.environment.clone().unwrap_or_default(),
                        volumes: app_req.volumes.clone().unwrap_or_default(),
                        agent_id: "current".to_string(),
                    };
                    
                    // Store the instance in our local state
                    app_manager.instances.lock().unwrap().insert(id, app_instance.clone());
                    
                    Ok(Json(app_instance))
                },
                Err(e) => Err(format!("Failed to start instance: {}", e))
            }
        },
        Err(e) => Err(format!("Failed to create instance: {}", e))
    }
}

#[put("/instances/<id>/start")]
pub async fn start_instance(id: String, app_manager: &State<AppManager>) -> Result<Json<AppInstance>, String> {
    // Start container
    match app_manager.docker.start_container(&id, None::<StartContainerOptions<String>>).await {
        Ok(_) => {
            // Get updated container info
            match get_instance(id, app_manager).await {
                Some(instance) => Ok(instance),
                None => Err("Failed to get instance after starting".to_string())
            }
        },
        Err(e) => Err(format!("Failed to start instance: {}", e))
    }
}

#[put("/instances/<id>/stop")]
pub async fn stop_instance(id: String, app_manager: &State<AppManager>) -> Result<Json<AppInstance>, String> {
    // Stop container
    let options = Some(StopContainerOptions {
        t: 30, // Give it 30 seconds to shut down gracefully
    });
    
    match app_manager.docker.stop_container(&id, options).await {
        Ok(_) => {
            // Get updated container info
            match get_instance(id, app_manager).await {
                Some(instance) => Ok(instance),
                None => Err("Failed to get instance after stopping".to_string())
            }
        },
        Err(e) => Err(format!("Failed to stop instance: {}", e))
    }
}

#[put("/instances/<id>/restart")]
pub async fn restart_instance(id: String, app_manager: &State<AppManager>) -> Result<Json<AppInstance>, String> {
    // Restart container
    let options = Some(bollard::container::RestartContainerOptions {
        t: 30, // Give it 30 seconds to shut down gracefully
    });
    
    match app_manager.docker.restart_container(&id, options).await {
        Ok(_) => {
            // Get updated container info
            match get_instance(id, app_manager).await {
                Some(instance) => Ok(instance),
                None => Err("Failed to get instance after restarting".to_string())
            }
        },
        Err(e) => Err(format!("Failed to restart instance: {}", e))
    }
}
#[patch("/instances/<id>", format = "json", data = "<update_req>")]
pub async fn update_instance(id: String, update_req: Json<AppInstanceRequest>, app_manager: &State<AppManager>) -> Result<Json<AppInstance>, String> {
    // For updating, we generally need to:
    // 1. Stop the existing container
    // 2. Remove it (but keep volumes if they're managed externally)
    // 3. Create a new one with the updated config
    // 4. Start it
    
    // This is a simplified implementation
    // In practice, you'd want to check what actually changed and handle it accordingly
    
    // First, stop the container
    let stop_result = stop_instance(id.clone(), app_manager).await;
    if stop_result.is_err() {
        return Err(format!("Failed to stop instance for update: {}", stop_result.err().unwrap()));
    }
    
    // Then remove it
    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });
    
    match app_manager.docker.remove_container(&id, options).await {
        Ok(_) => {
            // Now create a new one with the updated config
            create_instance(update_req, app_manager).await
        },
        Err(e) => Err(format!("Failed to remove instance for update: {}", e))
    }
}

#[delete("/instances/<id>")]
pub async fn delete_instance(id: String, app_manager: &State<AppManager>) -> Result<String, String> {
    // Remove container
    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });
    
    match app_manager.docker.remove_container(&id, options).await {
        Ok(_) => {
            // Remove from our local state
            app_manager.instances.lock().unwrap().remove(&id);
            Ok(format!("Instance {} deleted successfully", id))
        },
        Err(e) => Err(format!("Failed to delete instance: {}", e))
    }
}

#[get("/images")]
pub async fn list_images(app_manager: &State<AppManager>) -> Json<Vec<String>> {
    let mut images = Vec::new();
    
    // List images via Docker API
    let options = Some(ListImagesOptions::<String> {
        all: false,
        ..Default::default()
    });
    
    match app_manager.docker.list_images(options).await {
        Ok(image_list) => {
            for image in image_list {
                for tag in &image.repo_tags {
                    images.push(tag.clone());
                }
            }
        },
        Err(e) => {
            eprintln!("Failed to list images: {}", e);
        }
    }
    
    Json(images)
}

#[get("/events")]
pub async fn stream_events(app_manager: &State<AppManager>) -> String {
    // This would typically be implemented with Server-Sent Events or WebSockets
    // For this example, we'll just demonstrate the Docker events API
    
    let options = Some(EventsOptions::<String> {
        ..Default::default()
    });
    
    let mut event_stream = app_manager.docker.events(options);
    
    // In a real implementation, you'd stream these to the client
    // Here we'll just return a message
    while let Some(event) = event_stream.next().await {
        match event {
            Ok(event) => {
                println!("Event: {:?}", event);
                // In a real implementation, send this to the client
            },
            Err(e) => {
                eprintln!("Error receiving event: {}", e);
                break;
            }
        }
    }
    
    "Event streaming would happen here".to_string()
}

#[get("/health")]
pub fn health_check() -> String {
    "App Manager is healthy".to_string()
}