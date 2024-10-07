use std::env;
use std::fs;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Dependency {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub exports: String,
    pub dependencies_store: Option<String>,
    pub dependencies: Option<Vec<Dependency>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub projects: Vec<Project>,
}

pub fn find_path_in_hierarchy(file_name: &str) -> Option<String> {
    let mut current_dir = env::current_dir().expect("Erro ao obter diretório atual");

    while current_dir.pop() {
        let config_path = current_dir.join(file_name);

        if config_path.exists() {
            return Some(config_path.to_string_lossy().to_string());
        }
    }
    None
}

pub fn get_config_file() -> Config {
    let config_file = "fantoche.yaml";
    match find_path_in_hierarchy(config_file) {
        Some(config_path) => {
            let config_data =
                fs::read_to_string(config_path).expect("Err, cannot read config file");
            let config: Config =
                serde_yaml::from_str(&config_data).expect("Err on deserializing YAML");

            config
        }
        None => {
            println!("Configuration file not found");
            Config {
                projects: Vec::new(),
            }
        }
    }
}
