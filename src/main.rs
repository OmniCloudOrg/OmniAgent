use rocket::{get, post, delete, routes, serde::json::Json};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use sysinfo::{System};
use std::collections::HashMap;
use console::{style, Term};

const BANNER: &str = r#"
   ____  __  __ _   _ _____             _____  ______ _   _ _______ 
  / __ \|  \/  | \ | |_   _|      /\   / ____||  ____| \ | |__   __|
 | |  | | \  / |  \| | | |       /  \ | |  __|| |__  |  \| |  | |   
 | |  | | |\/| | . ` | | |      / /\ \| | |_ ||  __| | . ` |  | |   
 | |__| | |  | | |\  |_| |_    / ____ \ |__| || |____| |\  |  | |   
  \____/|_|  |_|_| \_|_____|  /_/    \_\_____||______|_| \_|  |_|   v{}"#;


#[macro_use] extern crate rocket;

const DOCKER_API_URL: &str = "http://localhost:2375"; // Docker daemon API endpoint

#[derive(Serialize, Deserialize)]
struct ContainerConfig {
    image: String,
    name: Option<String>,
    env: Option<Vec<String>>,
    ports: Option<HashMap<String, String>>, // "8080/tcp" -> "80"
    volumes: Option<Vec<String>>, // "/host/path:/container/path"
    network: Option<String>,
}

#[post("/deploy", data = "<config>")]
async fn deploy_container(config: Json<ContainerConfig>) -> Json<HashMap<String, String>> {
    let client = Client::new();
    let mut request_body = serde_json::json!({
        "Image": config.image,
        "HostConfig": {
            "Binds": config.volumes.clone().unwrap_or_default(),
            "NetworkMode": config.network.clone().unwrap_or("bridge".to_string()),
        }
    });

    if let Some(env) = &config.env {
        request_body["Env"] = serde_json::json!(env);
    }

    let response = client.post(format!("{}/containers/create", DOCKER_API_URL))
        .json(&request_body)
        .send().await.unwrap().json::<HashMap<String, String>>().await.unwrap();
    
    Json(response)
}

#[post("/start/<container_id>")]
async fn start_container(container_id: String) -> Json<HashMap<String, String>> {
    let client = Client::new();
    let response = client.post(format!("{}/containers/{}/start", DOCKER_API_URL, container_id))
        .send().await.unwrap();
    
    Json(HashMap::from([("status".to_string(), response.status().to_string())]))
}

#[post("/stop/<container_id>")]
async fn stop_container(container_id: String) -> Json<HashMap<String, String>> {
    let client = Client::new();
    let response = client.post(format!("{}/containers/{}/stop", DOCKER_API_URL, container_id))
        .send().await.unwrap();
    
    Json(HashMap::from([("status".to_string(), response.status().to_string())]))
}

#[delete("/remove/<container_id>")]
async fn remove_container(container_id: String) -> Json<HashMap<String, String>> {
    let client = Client::new();
    let response = client.delete(format!("{}/containers/{}", DOCKER_API_URL, container_id))
        .send().await.unwrap();
    
    Json(HashMap::from([("status".to_string(), response.status().to_string())]))
}

#[get("/status/<container_id>")]
async fn get_container_status(container_id: String) -> Json<HashMap<String, String>> {
    let client = Client::new();
    let response = client.get(format!("{}/containers/{}/json", DOCKER_API_URL, container_id))
        .send().await.unwrap().json::<HashMap<String, String>>().await.unwrap();
    
    Json(response)
}

#[post("/network/create/<network_name>")]
async fn create_network(network_name: String) -> Json<HashMap<String, String>> {
    let client = Client::new();
    let response = client.post(format!("{}/networks/create", DOCKER_API_URL))
        .json(&serde_json::json!({"Name": network_name}))
        .send().await.unwrap().json::<HashMap<String, String>>().await.unwrap();
    
    Json(response)
}

#[get("/networks")]
async fn list_networks() -> Json<Vec<HashMap<String, String>>> {
    let client = Client::new();
    let response = client.get(format!("{}/networks", DOCKER_API_URL))
        .send().await.unwrap().json::<Vec<HashMap<String, String>>>().await.unwrap();
    
    Json(response)
}

#[delete("/network/remove/<network_id>")]
async fn remove_network(network_id: String) -> Json<HashMap<String, String>> {
    let client = Client::new();
    let response = client.delete(format!("{}/networks/{}", DOCKER_API_URL, network_id))
        .send().await.unwrap();
    
    Json(HashMap::from([("status".to_string(), response.status().to_string())]))
}

#[get("/metrics/system")] 
fn stream_system_metrics() -> Json<HashMap<String, String>> {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let mut metrics = HashMap::new();
    metrics.insert("cpu_usage".to_string(), format!("{:.2}%", sys.global_cpu_usage()));
    metrics.insert("memory_usage".to_string(), format!("{:.2} MB", sys.used_memory() as f64 / 1024.0 / 1024.0));
    metrics.insert("total_memory".to_string(), format!("{:.2} MB", sys.total_memory() as f64 / 1024.0 / 1024.0));
    metrics.insert("disk_usage".to_string(), format!("{:.2} MB", sys.used_swap() as f64 / 1024.0 / 1024.0));
    
    Json(metrics)
}

#[launch]
fn rocket() -> _ {
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
    println!("  {} {}", style("API:").yellow().bold(), style("http://localhost:8081/api").white());
    println!("  {} {}", style("METRICS:").yellow().bold(), style("http://localhost:8081/metrics").white());
    println!("  {} {}", style("DOCKER TCP:").yellow().bold(), style("localhost:2375").white());
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
        

    rocket::build().mount("/", routes![
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
