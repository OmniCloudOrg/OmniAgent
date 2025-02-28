use omniagent::docker::DockerManager;
use omniagent::error::OmniAgentResult;
use rocket::{get, routes, serde::json::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use console::{style, Term};

const BANNER: &str = r#"
   ____  __  __ _   _ _____             _____ ______ _   _ _______ 
  / __ \|  \/  | \ | |_   _|      /\   / ____|  ____| \ | |__   __|
 | |  | | \  / |  \| | | |       /  \ | |  __| |__  |  \| |  | |   
 | |  | | |\/| | . ` | | |      / /\ \| | |_ |  __| | . ` |  | |   
 | |__| | |  | | |\  |_| |_    / ____ \ |__| | |____| |\  |  | |   
  \____/|_|  |_|_| \_|_____|  /_/    \_\_____|______|_| \_|  |_|   v{}"#;

#[derive(Serialize, Deserialize)]
struct VersionResponse {
    version: String,
    api_version: String,
    os: String,
    arch: String,
}

struct AppState {
    docker_manager: Arc<Mutex<DockerManager>>,
}

#[get("/version")]
async fn get_version(state: &rocket::State<AppState>) -> Json<VersionResponse> {
    let docker_manager = state.docker_manager.lock().await;
    
    match docker_manager.get_version().await {
        Ok(version_info) => {
            info!("Successfully retrieved Docker version");
            Json(VersionResponse {
                version: version_info.version.unwrap_or_else(|| "unknown".to_string()),
                api_version: version_info.api_version.unwrap_or_else(|| "unknown".to_string()),
                os: version_info.os.unwrap_or_else(|| "unknown".to_string()),
                arch: version_info.arch.unwrap_or_else(|| "unknown".to_string()),
            })
        },
        Err(e) => {
            error!("Failed to get Docker version: {}", e);
            // Return a placeholder response in case of error
            Json(VersionResponse {
                version: format!("Error: {}", e),
                api_version: "unknown".to_string(),
                os: "unknown".to_string(),
                arch: "unknown".to_string(),
            })
        }
    }
}

async fn print_startup_message() {
    let term = Term::stdout();
    let _ = term.clear_screen();
    
    // Print banner with version
    println!("{}", style(BANNER.replace("{}", env!("CARGO_PKG_VERSION"))).cyan().bold());
    println!();
    
    // Print startup information
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    println!("{} {}", style("▶").green().bold(), style("STARTING DOCKER AGENT").white().bold());
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    println!();
}

async fn print_status_message(message: &str, success: bool) {
    let status_symbol = if success { "✓" } else { "✗" };
    let status_style = if success { style(status_symbol).green().bold() } else { style(status_symbol).red().bold() };
    
    println!("  {} {}", status_style, style(message).white());
}

#[rocket::main]
async fn main() -> OmniAgentResult<()> {
    // Print startup banner before initializing logging
    print_startup_message().await;
    
    // Initialize logging with tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omniagent=info".into()),
        )
        .with_file(true)
        .with_line_number(true)
        .init();

    // Initialize Docker manager
    println!("{}", style("INITIALIZING SERVICES").yellow().bold());
    let docker_manager = match DockerManager::new().await {
        Ok(manager) => {
            print_status_message("Docker manager initialized successfully", true).await;
            manager
        },
        Err(e) => {
            print_status_message(&format!("Docker initialization failed: {}", e), false).await;
            println!("  {} {}", style("⟳").yellow().bold(), style("Attempting automatic initialization...").white());
            
            match DockerManager::initialize().await {
                Ok(manager) => {
                    print_status_message("Docker successfully initialized", true).await;
                    manager
                }
                Err(e) => {
                    print_status_message(&format!("Automatic initialization failed: {}", e), false).await;
                    return Err(e);
                }
            }
        }
    };

    let docker_manager = Arc::new(Mutex::new(docker_manager));
    let app_state = AppState { docker_manager };
    
    // Start the Rocket server
    println!();
    println!("{}", style("STARTING API SERVER").yellow().bold());
    print_status_message("Configuring Rocket API endpoints", true).await;
    
    println!();
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    println!("{} {}", 
        style("▶").green().bold(), 
        style("API SERVER READY").white().bold()
    );
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    println!("  {} {}", style("URL:").yellow().bold(), style("http://localhost:8000/api").white());
    println!("  {} {}", style("ENDPOINTS:").yellow().bold(), style("/version").white());
    println!("{}", style("═════════════════════════════════════════════════════").cyan());
    
    let _ = rocket::build()
        .manage(app_state)
        .mount("/api", routes![get_version])
        .launch()
        .await
        .expect("Failed to launch Rocket server");

    Ok(())
}