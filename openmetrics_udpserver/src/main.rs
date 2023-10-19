mod config;
mod http_server;
mod metrics;
mod processor;
mod udp_server;

use crate::config::Config;
use crate::processor::{InboundMetric, Processor};
use crate::udp_server::UdpServer;
use anyhow::{anyhow, Context};
use clap::{Arg, ArgAction, Command};
use prometheus_client::registry::Registry;
use std::sync::mpsc::channel;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    let matches = Command::new("Prometheus UDP Monitor")
        .version("2.0")
        .about("UDP Sender for Prometheus")
        .arg(
            Arg::new("udp-bind")
                .long("udp-bind")
                .default_value("127.0.0.1:1113")
                .help("UDP Server Bind Address.")
                .required(false),
        )
        .arg(
            Arg::new("http-bind")
                .long("http-bind")
                .default_value("127.0.0.1:1114")
                .help("HTTP Server Bind Address.")
                .required(false),
        )
        .arg(
            Arg::new("debug")
                .short('v')
                .help("verbose mode, just for debugging")
                .long("debug")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let config = Config {
        debug: matches.get_flag("debug"),
        udp_bind: matches
            .get_one::<String>("udp-bind")
            .ok_or(anyhow!("UDP bind host is missing"))?
            .to_string(),
        http_bind: matches
            .get_one::<String>("http-bind")
            .ok_or(anyhow!("HTTP bind host is missing"))?
            .to_string(),
    };

    println!("UDB Monitor for Prometheus");
    println!("debug: {:?}", &config.debug);
    println!("udp host: {}", &config.udp_bind);
    println!("http host: {}", &config.http_bind);

    let metric_registry = Arc::new(RwLock::new(Registry::default()));
    let (sender, receiver) = channel::<InboundMetric>();

    let processor_config = config.clone();
    let processor_registry = metric_registry.clone();
    tokio::spawn(async move {
        let mut processor = Processor::new(processor_config, processor_registry);
        processor
            .run(receiver)
            .expect("Issue running metric processor");
    });

    let udp_server_config = config.clone();
    tokio::spawn(async move {
        let udp_server = UdpServer::new(udp_server_config, sender);
        udp_server
            .run()
            .context("Error running UDP server loop")
            .unwrap();
    });

    http_server::bind(config, metric_registry).await?;
    Ok(())
}
