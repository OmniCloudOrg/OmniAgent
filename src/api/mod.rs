use anyhow::Result;
use debug_print::{
    debug_eprint as deprint,
    debug_eprintln as deprintln,
    debug_print as dprint,
    debug_println as dprintln,
};
use ez_logging::println;
use rocket::{ self, launch, post, response::Responder, routes, serde::json::Json, State };
use serde::{ Deserialize, Serialize };
use std::env;
use crate::cpi_actions::{ CpiCommand, CpiCommandType };
// Trait for container management operations

fn cpi_command(command: String) {

}
struct Docker;


impl Container for Docker {
    fn start(id: String) {
        run_cpi_command("start");

    }
    fn remove(id: String) {
        run_cpi_command("remove");
    }
}
trait Container {
    fn deploy(app_name: String, image_name: String, port: u32);
    fn remove(id: String);
    fn update(id: String);
    fn start(id: String);
    fn restart(id: String);
    fn stop(id: String);
    // fn attach_volume(id: String);  // TODO: These will be added when network-based file persistance is being implemented
    // fn detach_volume(id: String);  // TODO: These will be added when network-based file persistance is being implemented
    fn connect_network(id: String, app_name: String);
    fn configure_health_check(id: String);
    fn query_logs(id: String);
    fn configure(command: String) -> Result<()>;
}

// Custom error handling
#[derive(Debug, Responder)]
enum ApiError {
    #[response(status = 500)] Internal(String),
    #[response(status = 400)] BadRequest(String),
    #[response(status = 404)] NotFound(String),
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

// Additional types for container management
#[derive(Debug, Serialize, Deserialize)]
struct ContainerConfig {
    image: String,
    name: String,
    environment: Option<Vec<String>>,
    ports: Option<Vec<u16>>,
    volumes: Option<Vec<String>>,
    memory_limit: Option<String>,
    cpu_limit: Option<f32>,
    restart_policy: Option<String>,
    network: Option<String>,
    health_check: Option<HealthCheck>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthCheck {
    command: String,
    interval_seconds: u32,
    timeout_seconds: u32,
    retries: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScalingConfig {
    min_replicas: u32,
    max_replicas: u32,
    target_cpu_utilization: f32,
    cooldown_period: u32,
}

// Deployment-related operations
#[post("/containers/deploy", format = "json", data = "<params>")]
async fn deploy(params: Json<ContainerConfig>) -> ApiResult<String> {
    println!("Attempting to deploy container. Received params: {params:?}");
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::Deploy(params.into_inner()))?;
    Ok(Json(result.to_string()))
}

#[post("/containers/update", format = "json", data = "<params>")]
async fn update(params: Json<ContainerConfig>) -> ApiResult<String> {
    println!("Updating container configuration. Params: {params:?}");
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::Update(params.into_inner()))?;
    Ok(Json(result.to_string()))
}

#[post("/containers/<id>/stop")]
async fn stop(id: String) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::Stop(id))?;
    Ok(Json(result.to_string()))
}

#[post("/containers/<id>/start")]
async fn start(id: String) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::Start(id))?;
    Ok(Json(result.to_string()))
}

#[post("/containers/<id>/restart")]
async fn restart(id: String) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::Restart(id))?;
    Ok(Json(result.to_string()))
}

#[post("/containers/<id>/delete")]
async fn delete(id: String) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::Delete(id))?;
    Ok(Json(result.to_string()))
}

// Scaling operations
#[post("/containers/<id>/scale", format = "json", data = "<params>")]
async fn scale(id: String, params: Json<ScalingConfig>) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::Scale {
        id,
        config: params.into_inner(),
    })?;
    Ok(Json(result.to_string()))
}

#[post("/containers/<id>/autoscale", format = "json", data = "<params>")]
async fn configure_autoscaling(id: String, params: Json<ScalingConfig>) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::ConfigureAutoscaling {
        id,
        config: params.into_inner(),
    })?;
    Ok(Json(result.to_string()))
}

// Network operations
#[post("/containers/<id>/network/connect", format = "json", data = "<network>")]
async fn connect_network(id: String, network: Json<String>) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::ConnectNetwork {
        container_id: id,
        network: network.into_inner(),
    })?;
    Ok(Json(result.to_string()))
}

#[post("/containers/<id>/network/disconnect", format = "json", data = "<network>")]
async fn disconnect_network(id: String, network: Json<String>) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::DisconnectNetwork {
        container_id: id,
        network: network.into_inner(),
    })?;
    Ok(Json(result.to_string()))
}

// Volume operations
#[post("/containers/<id>/volumes/attach", format = "json", data = "<volume>")]
async fn attach_volume(id: String, volume: Json<String>) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::AttachVolume {
        container_id: id,
        volume: volume.into_inner(),
    })?;
    Ok(Json(result.to_string()))
}

#[post("/containers/<id>/volumes/detach", format = "json", data = "<volume>")]
async fn detach_volume(id: String, volume: Json<String>) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::DetachVolume {
        container_id: id,
        volume: volume.into_inner(),
    })?;
    Ok(Json(result.to_string()))
}

// Health and monitoring operations
#[post("/containers/<id>/health-check", format = "json", data = "<config>")]
async fn configure_health_check(id: String, config: Json<HealthCheck>) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::ConfigureHealthCheck {
        container_id: id,
        config: config.into_inner(),
    })?;
    Ok(Json(result.to_string()))
}

// Log operations
#[post("/containers/<id>/logs/stream")]
async fn stream_logs(id: String) -> ApiResult<String> {
    let cpi = CpiCommand::new()?;
    let result = cpi.execute(CpiCommandType::StreamLogs(id))?;
    Ok(Json(result.to_string()))
}

pub async fn rocket() -> rocket::Rocket<rocket::Build> {
    // Load environment variables
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    println!("Container Management API running at http://{}:{}", host, &port);

    // Configure Rocket
    let config = rocket::Config
        ::figment()
        .merge(("address", host))
        .merge(("port", port.parse::<u16>().unwrap()));

    rocket
        ::custom(config)
        .mount(
            "/",
            routes![
                deploy,
                update,
                stop,
                start,
                restart,
                delete,
                scale,
                configure_autoscaling,
                connect_network,
                disconnect_network,
                attach_volume,
                detach_volume,
                configure_health_check,
                stream_logs
            ]
        )
}

pub async fn launch_rocket() {
    rocket().await.launch().await.unwrap();
}
