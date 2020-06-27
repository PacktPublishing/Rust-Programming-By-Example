use std::fs::File;
use std::path::Path;
use std::io::{Read, Write};

use toml;

pub const DEFAULT_PORT: u16 = 1234;

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub server_port: Option<u16>,
    pub server_addr: Option<String>,
    pub admin: Option<User>,
    pub users: Vec<User>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub password: String,
}

fn get_content<P: AsRef<Path>>(file_path: &P) -> Option<String> {
    let mut file = File::open(file_path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    Some(content)
}

impl Config {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Option<Config> {
        if let Some(content) = get_content(&file_path) {
            toml::from_str(&content).ok()
        } else {
            println!("No config file found so creating a new one in {}",
                     file_path.as_ref().display());
            // In case we didn't find the config file, we just build a new one.
            let config = Config {
                server_port: Some(DEFAULT_PORT),
                server_addr: Some("127.0.0.1".to_owned()),
                admin: None,
                users: vec![User {
                    name: "anonymous".to_owned(),
                    password: "".to_owned(),
                }],
            };
            let content = toml::to_string(&config).expect("serialization failed");
            let mut file = File::create(file_path.as_ref()).expect("couldn't create file...");
            writeln!(file, "{}", content).expect("couldn't fulfill config file...");
            Some(config)
        }
    }
}
