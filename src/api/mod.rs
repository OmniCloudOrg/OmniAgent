use anyhow::Result;
use ez_logging::println;
use rocket::{post, response::Responder, routes, serde::json::Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, env};
use crate::cpi_actions::{CpiCommand, CpiCommandType};

// Container struct for management operations
pub struct Container;

impl Container {
    pub fn deploy(app_name: String, image_name: String, port: u16) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        cmd.execute(CpiCommandType::CreateContainer {
            image: image_name,
            name: app_name,
            ports: vec![format!("{}:{}", port, port)],
            env: HashMap::new(),
        })
    }

    pub fn remove(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        cmd.execute(CpiCommandType::DeleteContainer { name })
    }

    pub fn start(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        cmd.execute(CpiCommandType::StartContainer { name })
    }

    pub fn restart(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        cmd.execute(CpiCommandType::RestartContainer { name })
    }

    pub fn stop(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        cmd.execute(CpiCommandType::StopContainer { name })
    }

    pub fn inspect(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        cmd.execute(CpiCommandType::InspectContainer { name })
    }

    pub fn list() -> Result<Value> {
        let cmd = CpiCommand::new()?;
        cmd.execute(CpiCommandType::ListContainers)
    }
}

// Custom error handling
#[derive(Debug, Responder)]
pub enum ApiError {
    #[response(status = 500)]
    Internal(String),
    #[response(status = 400)]
    BadRequest(String),
    #[response(status = 404)]
    NotFound(String),
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

type ApiResult<T> = std::result::Result<Json<T>, ApiError>;

// Container configuration types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerConfig {
    pub image: String,
    pub name: String,
    pub environment: Option<HashMap<String, String>>,
    pub ports: Option<Vec<u16>>,
}

// API endpoints
#[post("/containers/deploy", format = "json", data = "<params>")]
pub async fn deploy(params: Json<ContainerConfig>) -> ApiResult<Value> {
    println!("Attempting to deploy container. Received params: {params:?}");
    let result = Container::deploy(
        params.name.clone(),
        params.image.clone(),
        params.ports.as_ref()
            .and_then(|ports| ports.first().copied())
            .unwrap_or(80),
    )?;
    Ok(Json(result))
}

#[post("/containers/<name>/start")]
pub async fn start(name: String) -> ApiResult<Value> {
    let result = Container::start(name)?;
    Ok(Json(result))
}

#[post("/containers/<name>/stop")]
pub async fn stop(name: String) -> ApiResult<Value> {
    let result = Container::stop(name)?;
    Ok(Json(result))
}

#[post("/containers/<name>/restart")]
pub async fn restart(name: String) -> ApiResult<Value> {
    let result = Container::restart(name)?;
    Ok(Json(result))
}

#[post("/containers/<name>/delete")]
pub async fn delete(name: String) -> ApiResult<Value> {
    let result = Container::remove(name)?;
    Ok(Json(result))
}

#[post("/containers/<name>")]
pub async fn inspect(name: String) -> ApiResult<Value> {
    let result = Container::inspect(name)?;
    Ok(Json(result))
}

#[post("/containers")]
pub async fn list() -> ApiResult<Value> {
    let result = Container::list()?;
    Ok(Json(result))
}

pub fn rocket() -> rocket::Rocket<rocket::Build> {
    // Load environment variables
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    println!("Container Management API running at http://{}:{}", host, &port);

    // Configure Rocket
    let config = rocket::Config::figment()
        .merge(("address", host))
        .merge(("port", port.parse::<u16>().unwrap()));

    rocket::custom(config).mount(
        "/",
        routes![
            deploy,
            start,
            stop,
            restart,
            delete,
            inspect,
            list
        ],
    )
}

pub async fn launch_rocket() -> Result<rocket::Rocket<rocket::Ignite>> {
    Ok(rocket().launch().await?)
}