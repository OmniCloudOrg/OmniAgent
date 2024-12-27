use anyhow::Context;
use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Output};
use ez_logging::println;

pub struct CpiCommand {
    pub config: String,
}

impl CpiCommand {
    pub fn new() -> Result<Self> {
        let config_str = fs::read_to_string("./CPIs/cpi-docker-win.json")?;
        let config_json: Value = serde_json::from_str(&config_str)?;

        Ok(Self {
            config: config_json.to_string(),
        })
    }

    // Execute a CPI command by fetching the template from the CPI,
    // filling the params, and returning the output
    pub fn execute(&self, command: CpiCommandType) -> Result<Value> {
        
        // Parse config
        let config_json: Value =
            serde_json::from_str(&self.config).context("failed to deserialize json")?;
        
        let actions = config_json
            .get("actions")
            .context("'actions' was not defined in the config")?;

        let command_type = actions.get(command.to_string()).context(format!(
            "Command type not found for '{}'",
            command.to_string()
        ))?;
    
        // Get command template
        let command_template = command_type
            .get("command")
            .context("'command field not found for command type'")?
            .as_str()
            .unwrap();
        
//        panic!("Command template: {}", command_template);

        // Get the post-exec command templates if they exist
        let post_exec_templates = match command_type.get("post_exec") {
            Some(post_exec) => {
                post_exec
                    .as_array()
                    .context("Post exec commands found but were not an array")?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .context("post exec command was not a valid string")
                            .map(|s| s.to_string())
                    })
                    .collect::<Result<Vec<String>>>()?
            }
            None => Vec::new(),
        };
    
        // Serialize the enum variant to a JSON Value and extract params
        let params: Value = serde_json::to_value(&command).context("failed to serialize command")?;
        println!("Command type: {}", command.to_string().green());
        println!("Command parameters:");
        println!("Params {}", &params);

        // Identify Params and insert the corrent values into the command template
        let params = if params.as_object().map_or(true, |obj| obj.is_empty()) {
            &Map::new()
        } else {
            params
            .as_object()
            .and_then(|obj| obj.values().next())
            .and_then(|v| v.as_object())
            .context("failed to extract params from command")?
        };
    
        // Execute main command
        let mut command_str = replace_template_params(params, &mut command_template.to_string());
    
        let output = execute_shell_cmd(&mut command_str)?;
        
        // Check main command execution
        if !output.status.success() {
            let error_msg = String::from_utf8(output.stderr)
                .context("failed to parse stderr as UTF-8")?;
            return Err(anyhow::anyhow!(error_msg));
        }
    
        // Parse the output of the main command
        let output_str = String::from_utf8(output.stdout)
            .context("failed to parse stdout as UTF-8")?;
    
        // Execute post-exec commands if they exist
        if !post_exec_templates.is_empty() {
            for (_, post_exec_template) in post_exec_templates.iter().enumerate() {
                let mut post_exec_command = replace_template_params(params, &mut post_exec_template.to_string());
        
                let post_exec_output = execute_shell_cmd(&mut post_exec_command)?;
        
                if !post_exec_output.status.success() {
                    let error_msg = String::from_utf8(post_exec_output.stderr)
                        .context("failed to parse post-exec stderr as UTF-8")?;
                    return Err(anyhow::anyhow!(error_msg));
                }
            }
            println!("Post-exec commands executed successfully");
        } else {
            println!("No post-exec commands found");
        }

        let escaped_output_str = serde_json::to_string(&output_str).context("failed to escape output string")?;
        let json_output = serde_json::from_str(&format!(r#"{{"result": {}}}"#, escaped_output_str))?;
        Ok(json_output)
    }
}

fn execute_shell_cmd(command_str: &mut String) -> Result<Output> {
    // Split into first word and the rest
    let mut parts = command_str.splitn(2, ' ');
    let executable = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    let parts_sting: String = parts.into_iter().collect();

    eprintln!("Executable: {:#}", executable);
    eprintln!("Args: {:#}", args);
    eprintln!("Parts: {:#}", parts_sting);

    let output = Command::new(executable)
        .args(args.split_whitespace())
        .output()?;

    Ok(output)
}

fn replace_template_params(params: &Map<String, Value>, command_str: &mut String) -> String {
    // Iterate through parameters and perform replacements
    for (key, value) in params {
        let placeholder = format!("{{{}}}", key); // Creates {key} format 
        let replacement = match value {
            Value::String(s) => s.to_owned(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Object(obj) => serde_json::to_string(&obj)
                .unwrap_or_default()
                .trim_matches(|c| c == '{' || c == '}')
                .to_string(),
            Value::Array(arr) => serde_json::to_string(&arr)
                .unwrap_or_default()
                .trim_matches(|c| c == '[' || c == ']')
                .to_string(),
            Value::Null => "null".to_string(),
        };

        *command_str = command_str.replace(&placeholder, &replacement);
    }
    command_str.to_string()
}

// Helper trait to handle special types
#[allow(dead_code)]
pub trait TemplateValue {
    fn to_template_string(&self) -> String;
}

// Implement for HashMap to handle networks
impl<K: ToString, V: ToString> TemplateValue for HashMap<K, V> {
    fn to_template_string(&self) -> String {
        self.iter()
            .map(|(k, v)| format!("{}={}", k.to_string(), v.to_string()))
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CpiCommandType {
    #[serde(rename = "create_container")]
    CreateContainer {
        image: String,
        name: String,
        ports: Vec<String>,
        env: HashMap<String, String>,
    },
    #[serde(rename = "delete_container")]
    DeleteContainer {
        name: String,
    },
    #[serde(rename = "start_container")]
    StartContainer {
        name: String,
    },
    #[serde(rename = "stop_container")]
    StopContainer {
        name: String,
    },
    #[serde(rename = "restart_container")]
    RestartContainer {
        name: String,
    },
    #[serde(rename = "inspect_container")]
    InspectContainer {
        name: String,
    },
    #[serde(rename = "list_containers")]
    ListContainers,
}

impl ToString for CpiCommandType {
    fn to_string(&self) -> String {
        match self {
            CpiCommandType::CreateContainer { .. } => "create_container".to_string(),
            CpiCommandType::DeleteContainer { .. } => "delete_container".to_string(),
            CpiCommandType::StartContainer { .. } => "start_container".to_string(),
            CpiCommandType::StopContainer { .. } => "stop_container".to_string(),
            CpiCommandType::RestartContainer { .. } => "restart_container".to_string(),
            CpiCommandType::InspectContainer { .. } => "inspect_container".to_string(),
            CpiCommandType::ListContainers => "list_containers".to_string(),
        }
    }
}

// Return types for the API calls
#[derive(Debug, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub state: String,
    pub image: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerList {
    pub containers: Vec<Container>,
}

#[allow(dead_code)]
pub struct CpiApi {
    cmd: CpiCommand,
}

#[allow(dead_code)]
pub fn test() {
    let cpi = CpiCommand::new().unwrap();
    let container = cpi.execute(CpiCommandType::CreateContainer {
        image: "nginx:latest".to_string(),
        name: "test-container".to_string(),
        ports: vec!["80:80".to_string()],
        env: HashMap::new(),
    });
    println!("Created Container: {:#?}", container);


    // Start the container
    let start_container = cpi.execute(CpiCommandType::StartContainer {
        name: "test-container".to_string(),
    });
    println!("Started Container: {:#?}", start_container);
    
    // Inspect the container
    let inspect_container = cpi.execute(CpiCommandType::InspectContainer {
        name: "test-container".to_string(),
    });
    println!("Inspected Container: {:#?}", inspect_container);
    
    // Stop the container
    let stop_container = cpi.execute(CpiCommandType::StopContainer {
        name: "test-container".to_string(),
    });
    println!("Stopped Container: {:#?}", stop_container);
    
    // Delete the container
    let delete_container = cpi.execute(CpiCommandType::DeleteContainer {
        name: "test-container".to_string(),
    });
    println!("Deleted Container: {:#?}", delete_container);
}
