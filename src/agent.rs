use uuid::Uuid;
use std::result::Result;

pub struct Agent {
    id: Uuid,
    name: String,
    version: String,
}

impl Agent {
    pub fn new(name: String, version: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            version,
        }
    }
    
    pub fn id(&self) -> Uuid {
        self.id
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn version(&self) -> &str {
        &self.version
    }
    pub async fn start() -> Result<Self, std::io::Error> {
        let agent = Agent::new("OmniAgent".to_string(), env!("CARGO_PKG_VERSION").to_string());
        Ok(agent)
    }
}