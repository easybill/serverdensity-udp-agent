use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Clone, Debug)]
pub struct Config {
    pub token: String,
    pub account_url: String,
    pub agent_key: String,
    pub serverdensity_endpoint: String,
    pub debug: bool,
    pub bind: String
}

impl Config {

    fn line_value(&self, line : &str) -> String {
        let value = line.trim().split(":").map(|x|x.trim().to_string()).collect::<Vec<String>>();
        
        if value.len() != 2 {
            return "".to_string();
        }

        return value[1].clone();
    }

    pub fn apply_config_file(&mut self, filename: &str) -> Result<(), String> {
        let file = File::open(filename).or_else(|_| Err("could not open config file".to_string()))?;
        let mut buf_reader = BufReader::new(file);
        
        let mut content = String::new();
        
        buf_reader.read_to_string(&mut content).or_else(|_| Err("could not read config file".to_string()))?;

        for line in content.split("\n") {
            if line.trim().starts_with("#") || line.trim().starts_with("[")  {
                continue;
            }

            if line.trim().starts_with("agent_key") {
                self.agent_key = self.line_value(&line);
                continue;
            }

            if line.trim().starts_with("sd_account") {
                self.account_url = self.line_value(&line);
                continue;
            }
        }

        Ok(())
    }
}