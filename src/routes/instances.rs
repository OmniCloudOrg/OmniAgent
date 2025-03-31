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
use futures::stream::{StreamExt, TryStreamExt};

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

#[get("/instances/<id>/logs")]
pub async fn get_instance_logs(id: String, app_manager: &State<AppManager>) -> Result<String, String> {
    let options = Some(bollard::container::LogsOptions::<String> {
        stdout: true,
        stderr: true,
        follow: false,
        timestamps: true,
        tail: "100".to_string(),
        ..Default::default()
    });

    match app_manager.docker.logs(&id, options).try_collect::<Vec<_>>().await {
        Ok(logs) => {
            let log_content = logs.iter()
                .map(|chunk| {
                    match chunk {
                        bollard::container::LogOutput::StdOut { message: bytes } | 
                        bollard::container::LogOutput::StdErr { message: bytes } => {
                            String::from_utf8_lossy(bytes).to_string()
                        },
                        bollard::container::LogOutput::StdIn { message: bytes } => {
                            String::from_utf8_lossy(bytes).to_string()
                        },
                        bollard::container::LogOutput::Console { message: bytes } => {
                            String::from_utf8_lossy(bytes).to_string()
                        }
                    }
                })
                .collect::<Vec<String>>()
                .join("");
            Ok(log_content)
        },
        Err(e) => Err(format!("Failed to fetch logs: {}", e))
    }
}

#[get("/instances/<id>/stats")]
pub async fn get_instance_stats(id: String, app_manager: &State<AppManager>) -> Result<Json<bollard::container::Stats>, String> {
    match app_manager.docker.stats(&id, Some(bollard::container::StatsOptions { 
        stream: false,
        one_shot: true,
    })).try_next().await {
        Ok(Some(stats)) => Ok(Json(stats)),
        Ok(None) => Err("No stats available".to_string()),
        Err(e) => Err(format!("Failed to get stats: {}", e))
    }
}

#[put("/instances/<id>/pause")]
pub async fn pause_instance(id: String, app_manager: &State<AppManager>) -> Result<String, String> {
    match app_manager.docker.pause_container(&id).await {
        Ok(_) => Ok(format!("Instance {} paused", id)),
        Err(e) => Err(format!("Failed to pause instance: {}", e))
    }
}

#[put("/instances/<id>/unpause")]
pub async fn unpause_instance(id: String, app_manager: &State<AppManager>) -> Result<String, String> {
    match app_manager.docker.unpause_container(&id).await {
        Ok(_) => Ok(format!("Instance {} unpaused", id)),
        Err(e) => Err(format!("Failed to unpause instance: {}", e))
    }
}

#[get("/instances/<id>/inspect")]
pub async fn inspect_instance(id: String, app_manager: &State<AppManager>) -> Result<Json<bollard::models::ContainerInspectResponse>, String> {
    match app_manager.docker.inspect_container(&id, None).await {
        Ok(info) => Ok(Json(info)),
        Err(e) => Err(format!("Failed to inspect instance: {}", e))
    }
}

// Volume Management

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    name: String,
    mountpoint: String,
    labels: HashMap<String, String>,
    created_at: String,
}

#[get("/volumes")]
pub async fn list_volumes(app_manager: &State<AppManager>) -> Result<Json<Vec<VolumeInfo>>, String> {
    match app_manager.docker.list_volumes::<String>(None).await {
        Ok(volumes) => {
            let volume_list = volumes.volumes.unwrap_or_default().into_iter()
                .filter_map(|vol| {
                    let name = vol.name;
                    let mountpoint = vol.mountpoint;
                    let labels = vol.labels;
                    let created_at = vol.created_at.unwrap_or_default();
                    
                    Some(VolumeInfo {
                        name,
                        mountpoint,
                        labels,
                        created_at,
                    })
                })
                .collect();
            
            Ok(Json(volume_list))
        },
        Err(e) => Err(format!("Failed to list volumes: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeCreateRequest {
    name: String,
    labels: Option<HashMap<String, String>>,
}

#[post("/volumes", format = "json", data = "<volume_req>")]
pub async fn create_volume(volume_req: Json<VolumeCreateRequest>, app_manager: &State<AppManager>) -> Result<Json<VolumeInfo>, String> {
    let options = bollard::volume::CreateVolumeOptions {
        name: volume_req.name.clone(),
        labels: volume_req.labels.clone().unwrap_or_default(),
        ..Default::default()
    };
    
    match app_manager.docker.create_volume(options).await {
        Ok(volume) => {
            let volume_info = VolumeInfo {
                name: volume.name,
                mountpoint: volume.mountpoint,
                labels: volume.labels,
                created_at: volume.created_at.unwrap_or_default(),
            };
            
            Ok(Json(volume_info))
        },
        Err(e) => Err(format!("Failed to create volume: {}", e))
    }
}

#[delete("/volumes/<name>")]
pub async fn delete_volume(name: String, app_manager: &State<AppManager>) -> Result<String, String> {
    match app_manager.docker.remove_volume(&name, None).await {
        Ok(_) => Ok(format!("Volume {} deleted successfully", name)),
        Err(e) => Err(format!("Failed to delete volume: {}", e))
    }
}

// Network Management

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    id: String,
    name: String,
    driver: String,
    scope: String,
    containers: HashMap<String, NetworkContainerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkContainerInfo {
    name: String,
    endpoint_id: String,
    ipv4_address: String,
}

#[get("/networks")]
pub async fn list_networks(app_manager: &State<AppManager>) -> Result<Json<Vec<NetworkInfo>>, String> {
    match app_manager.docker.list_networks::<String>(None).await {
        Ok(networks) => {
            let network_list = networks.into_iter()
                .filter_map(|net| {
                    let id = net.id?;
                    let name = net.name?;
                    let driver = net.driver?;
                    let scope = net.scope?;
                    
                    let mut containers = HashMap::new();
                    if let Some(net_containers) = net.containers {
                        for (container_id, container_info) in net_containers {
                            if let (Some(name), Some(endpoint_id), Some(ipv4_address)) = 
                               (container_info.name, container_info.endpoint_id, container_info.ipv4_address) {
                                containers.insert(container_id, NetworkContainerInfo {
                                    name,
                                    endpoint_id,
                                    ipv4_address,
                                });
                            }
                        }
                    }
                    
                    Some(NetworkInfo {
                        id,
                        name,
                        driver,
                        scope,
                        containers,
                    })
                })
                .collect();
            
            Ok(Json(network_list))
        },
        Err(e) => Err(format!("Failed to list networks: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCreateRequest {
    name: String,
    driver: Option<String>,
    labels: Option<HashMap<String, String>>,
}

#[post("/networks", format = "json", data = "<network_req>")]
pub async fn create_network(network_req: Json<NetworkCreateRequest>, app_manager: &State<AppManager>) -> Result<Json<NetworkInfo>, String> {
    let options = bollard::network::CreateNetworkOptions {
        name: network_req.name.clone(),
        driver: network_req.driver.clone().unwrap_or_default(),
        labels: network_req.labels.clone().unwrap_or_default(),
        ..Default::default()
    };
    
    match app_manager.docker.create_network(options).await {
        Ok(response) => {
            // Inspect network to get full details
            match app_manager.docker.inspect_network::<String>(response.id.as_str(), None).await {
                Ok(network) => {
                    let mut containers = HashMap::new();
                    if let Some(net_containers) = network.containers {
                        for (container_id, container_info) in net_containers {
                            if let (Some(name), Some(endpoint_id), Some(ipv4_address)) = 
                               (container_info.name, container_info.endpoint_id, container_info.ipv4_address) {
                                containers.insert(container_id, NetworkContainerInfo {
                                    name,
                                    endpoint_id,
                                    ipv4_address,
                                });
                            }
                        }
                    }
                    
                    let network_info = NetworkInfo {
                        id: network.id.unwrap_or_default(),
                        name: network.name.unwrap_or_default(),
                        driver: network.driver.unwrap_or_default(),
                        scope: network.scope.unwrap_or_default(),
                        containers,
                    };
                    
                    Ok(Json(network_info))
                },
                Err(e) => Err(format!("Failed to inspect created network: {}", e))
            }
        },
        Err(e) => Err(format!("Failed to create network: {}", e))
    }
}

#[delete("/networks/<id>")]
pub async fn delete_network(id: String, app_manager: &State<AppManager>) -> Result<String, String> {
    match app_manager.docker.remove_network(&id).await {
        Ok(_) => Ok(format!("Network {} deleted successfully", id)),
        Err(e) => Err(format!("Failed to delete network: {}", e))
    }
}

#[put("/instances/<id>/connect/<network_id>")]
pub async fn connect_instance_to_network(id: String, network_id: String, app_manager: &State<AppManager>) -> Result<String, String> {
    let options = bollard::network::ConnectNetworkOptions {
        container: id.clone(),
        ..Default::default()
    };
    
    match app_manager.docker.connect_network(&network_id, options).await {
        Ok(_) => Ok(format!("Instance {} connected to network {}", id, network_id)),
        Err(e) => Err(format!("Failed to connect instance to network: {}", e))
    }
}

#[put("/instances/<id>/disconnect/<network_id>")]
pub async fn disconnect_instance_from_network(id: String, network_id: String, app_manager: &State<AppManager>) -> Result<String, String> {
    let options = bollard::network::DisconnectNetworkOptions {
        container: id.clone(),
        force: false,
    };
    
    match app_manager.docker.disconnect_network(&network_id, options).await {
        Ok(_) => Ok(format!("Instance {} disconnected from network {}", id, network_id)),
        Err(e) => Err(format!("Failed to disconnect instance from network: {}", e))
    }
}

// Agent Management Routes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    id: String,
    name: String,
    version: String,
    platform: String,
    instance_count: usize,
    status: String,
    resources: SystemResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    cpu_count: usize,
    memory_total: u64,
    memory_available: u64,
    disk_total: u64,
    disk_available: u64,
}

#[get("/agent/info")]
pub async fn get_agent_info(app_manager: &State<AppManager>) -> Json<AgentInfo> {
    // Get Docker engine info
    let info = match app_manager.docker.info().await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Failed to get Docker info: {}", e);
            return Json(AgentInfo {
                id: uuid::Uuid::new_v4().to_string(),
                name: hostname::get().unwrap_or_default().to_string_lossy().to_string(),
                version: "unknown".to_string(),
                platform: "unknown".to_string(),
                instance_count: app_manager.instances.lock().unwrap().len(),
                status: "degraded".to_string(),
                resources: SystemResources {
                    cpu_count: num_cpus::get(),
                    memory_total: 0,
                    memory_available: 0,
                    disk_total: 0,
                    disk_available: 0,
                },
            });
        }
    };
    
    // Get system resources
    let memory_info = sys_info::mem_info().unwrap_or(sys_info::MemInfo {
        total: 0,
        free: 0,
        avail: 0,
        buffers: 0,
        cached: 0,
        swap_total: 0,
        swap_free: 0,
    });
    
    let disk_info = sys_info::disk_info().unwrap_or(sys_info::DiskInfo {
        total: 0,
        free: 0,
    });
    
    Json(AgentInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: hostname::get().unwrap_or_default().to_string_lossy().to_string(),
        version: info.server_version.unwrap_or_default(),
        platform: format!("{} / {}", 
            info.operating_system.unwrap_or_default(),
            info.architecture.unwrap_or_default()),
        instance_count: app_manager.instances.lock().unwrap().len(),
        status: "healthy".to_string(),
        resources: SystemResources {
            cpu_count: num_cpus::get(),
            memory_total: memory_info.total * 1024,
            memory_available: memory_info.avail * 1024,
            disk_total: disk_info.total * 1024,
            disk_available: disk_info.free * 1024,
        },
    })
}
