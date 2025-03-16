use rocket::{delete, get, post, patch};

#[get("/containers")]
pub fn get_containers() -> String {
    "Hello, world!".to_string()
}

#[post("/containers")]
pub fn create_container() -> String {
    "Hello, world!".to_string()
}

#[delete("/containers")]
pub fn delete_container() -> String {
    "Hello, world!".to_string()
}

#[patch("/containers")]
pub fn update_container() -> String {
    "Hello, world!".to_string()
}