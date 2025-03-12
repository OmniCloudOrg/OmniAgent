use colored::Colorize;
use rocket::routes;

mod agent;
use agent::Agent;

pub mod routes;


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
async fn main() {
    println!("{}", BANNER.replace("{}", &env!("CARGO_PKG_VERSION")));
    let agent = Agent::new("OmniAgent 1".to_string(), env!("CARGO_PKG_VERSION").to_string());
    println!("+-----------------------------------------------------------------");
    println!("| Selected UUID for agent: {}", agent.id().to_string().bright_green());
    println!("| Agent name: {}", agent.name().bright_blue());
    println!("+-----------------------------------------------------------------");

    let _server = rocket::build()
        .mount("/", routes![routes::index::index])
        .configure(rocket::Config {
            address: "0.0.0.0".parse().unwrap(),
            ..rocket::Config::default()
        })
        .manage(agent)
        .launch()
        .await;
}