use anyhow::Result;
use ez_logging::println;
use rocket::{post, get, response::Responder, serde::json::Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::cpi_actions::{CpiCommand, CpiCommandType};

#[derive(Deserialize, Debug)]
struct DockerResponse {
    result: String,
}

// Container struct for management operations
pub struct Container;

impl Container {
    fn parse_json_response(value: Value) -> Result<Value> {
        // First try to parse as a DockerResponse
        if let Ok(docker_response) = serde_json::from_value::<DockerResponse>(value.clone()) {            
            // Try to parse as JSON
            eprintln!("{:?}", docker_response);

            return serde_json::from_str(&docker_response.result)
                .map_err(|e| {
                    eprintln!("Failed to parse JSON: {}", e);
                    anyhow::anyhow!("Failed to parse JSON: {}\nJSON Data: {:?}", e, &docker_response.result)
                });
        }
        
        // If none of the above work, return the original value
        Ok(value)
    }

    pub fn deploy(app_name: String, image_name: String, port: u16) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        let response = cmd.execute(CpiCommandType::CreateContainer {
            image:  image_name,
            name:   app_name,
            ports:  vec![format!("{}:{}", port, port)],
            env:    HashMap::new(),
        })?;
        Self::parse_json_response(response)
    }

    pub fn remove(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        let response = cmd.execute(CpiCommandType::DeleteContainer { name })?;
        Self::parse_json_response(response)
    }

    pub fn start(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        let response = cmd.execute(CpiCommandType::StartContainer { name })?;
        Self::parse_json_response(response)
    }

    pub fn restart(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        let response = cmd.execute(CpiCommandType::RestartContainer { name })?;
        Self::parse_json_response(response)
    }

    pub fn stop(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        let response = cmd.execute(CpiCommandType::StopContainer { name })?;
        Self::parse_json_response(response)
    }

    pub fn inspect(name: String) -> Result<Value> {
        let cmd = CpiCommand::new()?;
        let response = cmd.execute(CpiCommandType::InspectContainer { name })?;
        Self::parse_json_response(response)
    }

    pub fn list() -> Result<Value> {
        let cmd = CpiCommand::new()?;
        let running = CpiCommandType::ListContainers;
        let response: Value = cmd.execute(running)?;
        Self::parse_json_response(response)
    }
}

// Custom error handling
#[allow(dead_code)]
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
    pub image:       String,
    pub name:        String,
    pub environment: Option<HashMap<String, String>>,
    pub ports:       Option<Vec<u16>>,
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

#[get("/containers/<name>")]
pub async fn inspect(name: String) -> ApiResult<Value> {
    let result = Container::inspect(name)?;
    Ok(Json(result))
}

#[get("/containers")]
pub async fn list() -> ApiResult<Value> {
    let result = Container::list()?;
    Ok(Json(result))
}

