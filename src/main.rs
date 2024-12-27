
use rocket::launch;
use serde::{Deserialize, Serialize};
use std::env;
use std::collections::HashMap;
use rocket::routes;
//mod metrics;
mod api;
mod cpi_actions;

//-----------------------------------------------------------------------------
// Data structures
//-----------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct Container {
    id:      String,
    name:    String,
    image:   String,
    status:  String,
    created: String,
    ports:   Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ContainerConfig {
    name:        String,
    image:       String,
    ports:       Vec<String>,
    environment: Option<HashMap<String, String>>,
}




//-----------------------------------------------------------------------------
// Router and server initialization
//-----------------------------------------------------------------------------
#[launch]
pub async fn rocket() -> rocket::Rocket<rocket::Build> {
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
            api::deploy,
            api::start,
            api::stop,
            api::restart,
            api::delete,
            api::inspect,
            api::list
        ],
    )
}

