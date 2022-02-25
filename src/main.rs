extern crate byteorder;
extern crate clap;
extern crate url;
extern crate md5;
extern crate reqwest;
extern crate regex;
extern crate openssl_probe;

mod config;
mod aggregator;
mod udpserver;
mod handler;

use clap::{Arg, App};
use std::sync::mpsc::channel;
use std::thread;
use serverdensity_udp_adgent::MetricType;
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

    ::openssl_probe::init_ssl_cert_env_vars();

    let matches = App::new("Server Density UDP Monitor")
        .version("1.0")
        .author("Tim Glabisch. <serverdensity@tim.ainfach.de>")
        .about("UDP Sender for Serverdendity")
        .arg(Arg::new("token")
            .help("Server Density API Token")
            .long("token")
            .required(true)
            .takes_value(true))
        .arg(Arg::new("account-url")
            .help("Set this to your Server Density account url, e.g. example.serverdensity.io")
            .long("account-url")
            .required(false)
            .takes_value(true))
        .arg(Arg::new("agent-key")
            .help("This is the agent key used to identify the device when payloads are processed. You can find this in the top left corner when you view a device page in your UI")
            .long("agent-key")
            .required(false)
            .takes_value(true))
        .arg(Arg::new("serverdensity-endpoint")
            .default_value("https://api.serverdensity.io")
            .help("Serverdensity API-Endpoint")
            .long("serverdensity-endpoint")
            .required(false)
            .takes_value(true))
        .arg(Arg::new("bind")
            .default_value("127.0.0.1:1113")
            .help("Bind Address.")
            .long("bind")
            .required(false)
            .takes_value(true))
        .arg(Arg::new("debug")
            .short('v')
            .help("verbose mode, just for debugging")
            .long("debug")
            .takes_value(false))
        .arg(Arg::new("config")
            .short('c')
            .help("path to the serverdensity config file, may /etc/sd-agent/config.cfg?")
            .long("config")
            .takes_value(true))
        .get_matches();

    let mut config = Config {
        token: matches.value_of("token").unwrap().to_string(),
        account_url: matches.value_of("account-url").unwrap_or("").to_string(),
        agent_key: matches.value_of("agent-key").unwrap_or("").to_string(),
        serverdensity_endpoint: matches.value_of("serverdensity-endpoint").unwrap().to_string(),
        debug: matches.is_present("debug"),
        bind: matches.value_of("bind").unwrap().to_string()
    };

    println!("UDP Sender for Serverdendity");
    
    if matches.is_present("config") {
        let config_file = matches.value_of("config").unwrap().to_string();
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