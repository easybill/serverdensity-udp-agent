extern crate byteorder;
extern crate clap;
extern crate url;
extern crate md5;
extern crate reqwest;
extern crate regex;

mod config;
mod aggregator;
mod udpserver;
mod handler;

use clap::{Arg, ArgAction, Command};
use std::sync::mpsc::channel;
use std::thread;
use serverdensity_udpserver_lib::MetricType;
use crate::config::Config;
use crate::aggregator::Aggregator;
use crate::udpserver::UdpServer;

#[derive(Debug)]
pub struct Metric {
    pub name: String,
    pub count: i32,
    pub metric_type: MetricType
}

fn main() {

    let matches = Command::new("Server Density UDP Monitor")
        .version("1.0")
        .author("Tim Glabisch. <serverdensity@tim.ainfach.de>")
        .about("UDP Sender for Serverdensity")
        .arg(Arg::new("token")
            .help("Server Density API Token")
            .long("token")
            .required(true))
        .arg(Arg::new("account-url")
            .help("Set this to your Server Density account url, e.g. example.serverdensity.io")
            .long("account-url")
            .required(false))
        .arg(Arg::new("agent-key")
            .help("This is the agent key used to identify the device when payloads are processed. You can find this in the top left corner when you view a device page in your UI")
            .long("agent-key")
            .required(false))
        .arg(Arg::new("serverdensity-endpoint")
            .default_value("https://api.serverdensity.io")
            .help("Serverdensity API-Endpoint")
            .long("serverdensity-endpoint")
            .required(false))
        .arg(Arg::new("bind")
            .default_value("127.0.0.1:1113")
            .help("Bind Address.")
            .long("bind")
            .required(false))
        .arg(Arg::new("debug")
            .short('v')
            .help("verbose mode, just for debugging")
            .long("debug")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("config")
            .short('c')
            .help("path to the serverdensity config file, may /etc/sd-agent/config.cfg?")
            .long("config"))
        .get_matches();

    let mut config = Config {
        token: matches.get_one::<String>("token").unwrap().to_string(),
        account_url: matches.get_one::<String>("account-url").unwrap_or(&"".to_string()).to_string(),
        agent_key: matches.get_one::<String>("agent-key").unwrap_or(&"".to_string()).to_string(),
        serverdensity_endpoint: matches.get_one::<String>("serverdensity-endpoint").unwrap().to_string(),
        debug: matches.get_flag("debug"),
        bind: matches.get_one::<String>("bind").unwrap().to_string()
    };

    println!("UDP Sender for Serverdendity");
    
    if matches.get_one::<String>("config").is_some() {
        let config_file = matches.get_one::<String>("config").unwrap().to_string();
        match config.apply_config_file(&config_file) {
            Ok(_) => println!("successfully read config_file: {}", &config_file),
            Err(_) => {
                println!("could not read config_file: {}", &config_file);
                return;
            }
        };
    }

    println!("account-url: {}", &config.account_url);
    println!("agent-key: {}", &config.agent_key);
    println!("endpoint: {}", &config.serverdensity_endpoint);
    println!("debug: {:?}", &config.debug);
    println!("bind: {}", &config.bind);

    if config.agent_key.trim() == "" || config.account_url.trim() == "" {
        println!("agent-key or account-url not given.");
        return;
    }

    println!("\nServer is starting...");

    let (sender, receiver) = channel::<Metric>();

    let thread_config = config.clone();
    thread::spawn(move|| {
        Aggregator::new(&thread_config).run(receiver)
    });

    UdpServer::new().run(&config, sender);
}