use rocket::{get, post};

#[get("/containers")]
pub fn get_containers() -> String {
    "Hello, world!".to_string()
}

#[post("/containers")]
pub fn create_container() -> String {
    "Hello, world!".to_string()
}