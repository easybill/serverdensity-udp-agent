mod config;
mod http_server;
mod metrics;
mod processor;
mod serverdensity;
mod udp_server;

use crate::config::Config;
use crate::processor::{InboundMetric, Processor};
use crate::serverdensity::aggregator::{ServerDensityAggregator, ServerDensityConfig};
use crate::udp_server::UdpServer;
use anyhow::{anyhow, Context};
use async_channel::unbounded;
use clap::{Arg, ArgAction, Command};
use prometheus_client::registry::Registry;
use std::process::exit;
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
        // ---- ServerDensity Args
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
        .arg(Arg::new("config")
            .short('c')
            .help("path to the serverdensity config file, may /etc/sd-agent/config.cfg?")
            .long("config"))
        // ---- ServerDensity Args
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

    println!("UDP Monitor for OpenMetrics");
    println!("debug: {:?}", &config.debug);
    println!("udp host: {}", &config.udp_bind);
    println!("http host: {}", &config.http_bind);

    let metric_registry = Arc::new(RwLock::new(Registry::default()));
    let (sender, receiver) = unbounded::<InboundMetric>();

    let processor_config = config.clone();
    let processor_receiver = receiver.clone();
    let processor_registry = metric_registry.clone();
    let processor_handle = tokio::spawn(async move {
        let mut processor = Processor::new(processor_config, processor_registry);
        processor.run(processor_receiver).await;
    });

    let udp_server_config = config.clone();
    let udp_server_handle = tokio::spawn(async move {
        let udp_server = UdpServer::new(udp_server_config, sender);
        udp_server.run().await;
    });

    // server density aggregator
    let server_density_aggregator_handle = tokio::spawn(async move {
        let server_density_config = ServerDensityConfig::from_args(matches);
        let server_density_aggregator = ServerDensityAggregator::new(server_density_config);
        server_density_aggregator.run(receiver).await;
    });

    // bind the http server to serve open metrics requests
    let http_server_registry = metric_registry.clone();
    let http_server_handle = http_server::bind(config, http_server_registry);

    // waits for one tasks to fail or interrupt, returns the status code to identity the issue
    let exit_code = tokio::spawn(async move {
        tokio::select! {
            _ = processor_handle => {
                eprintln!("Metrics processor failed");
                100
            }
            _ = udp_server_handle => {
                eprintln!("UDP server failed");
                101
            }
            _ = server_density_aggregator_handle => {
                eprintln!("Server Density aggregator failed");
                102
            }
            _ = http_server_handle => {
                eprintln!("Http server failed");
                103
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Quit signal detected, exiting...");
                0
            }
        }
    })
    .await
    .context("Error running main monitor loop")?;

    exit(exit_code)
}
