use colored::Colorize;
use rocket::routes;

pub mod routes;
use routes::{index, instances};
use routes::instances::AppManager;

mod agent;
use agent::Agent;



const BANNER: &str = r#"
   ____  __  __ _   _ _____             _____ ______ _   _ _______ 
  / __ \|  \/  | \ | |_   _|      /\   / ____|  ____| \ | |__   __|
 | |  | | \  / |  \| | | |       /  \ | |  __| |__  |  \| |  | |   
 | |  | | |\/| | . ` | | |      / /\ \| | |_ |  __| | . ` |  | |   
 | |__| | |  | | |\  |_| |_    / ____ \ |__| | |____| |\  |  | |   
  \____/|_|  |_|_| \_|_____|  /_/    \_\_____|______|_| \_|  |_|
                        Version: {}
"#;
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    println!("{}", BANNER.replace("{}", &env!("CARGO_PKG_VERSION")));
    let agent = Agent::new("OmniAgent 1".to_string(), env!("CARGO_PKG_VERSION").to_string());
    println!("+-----------------------------------------------------------------");
    println!("| Selected UUID for agent: {}", agent.id().to_string().bright_green());
    println!("| Agent name: {}", agent.name().bright_blue());
    println!("+-----------------------------------------------------------------");

    let routes = routes![
        index::     index,
        instances:: list_instances,
        instances:: get_instance,
        instances:: create_instance,
        instances:: start_instance,
        instances:: stop_instance,
        instances:: restart_instance,
        instances:: update_instance,
        instances:: delete_instance,
        instances:: list_images,
        instances:: stream_events,
        instances:: health_check

    ];

    let routes_clone = routes.clone();
    let app_manager = match AppManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to initialize AppManager: {}", e);
            std::process::exit(1);
        }
    };

    let rocket_instance = rocket::build()
        .mount("/", routes)
        .configure(rocket::Config {
            address: "0.0.0.0".parse().unwrap(),
            ..rocket::Config::default()
        })
        .manage(routes_clone)
        .manage(app_manager);

    // Collect routes information before launch
    index::collect_routes(&rocket_instance);
    
    // Launch the server
    let _server = rocket_instance.launch().await?;
    

    Ok(())
}