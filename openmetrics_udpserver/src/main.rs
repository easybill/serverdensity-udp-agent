mod aggregator;
mod config;
mod http_server;
mod processor;
mod serverdensity;
mod udp_server;

use crate::config::Config;
use crate::processor::{InboundMetric, Processor};
use crate::serverdensity::aggregator::{ServerDensityAggregator, ServerDensityConfig};
use crate::udp_server::UdpServer;
use anyhow::{anyhow, Context};
use clap::{Arg, ArgAction, Command};
use once_cell::sync::Lazy;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::registry::Registry;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::broadcast::channel;
use tokio::sync::RwLock;

pub static METRIC_COUNTER_REQUESTS: Lazy<Counter<u64>> = Lazy::new(Default::default);
pub static METRIC_COUNTER_ERRORS: Lazy<Counter<u64>> = Lazy::new(Default::default);
pub static METRIC_COUNTER_UDP_PACKETS: Lazy<Counter<u64>> = Lazy::new(Default::default);

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
        .arg(
            Arg::new("disable-serverdensity")
                .long("disable-serverdensity")
                .help("Disable ServerDensity push - only provide open metrics pull endpoint")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("token")
            .help("Server Density API Token")
            .long("token")
            .required(false))
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
        disable_serverdensity: matches.get_flag("disable-serverdensity"),
    };

    println!("UDP Monitor for OpenMetrics");
    println!("debug: {:?}", &config.debug);
    println!("udp host: {}", &config.udp_bind);
    println!("http host: {}", &config.http_bind);
    println!("disable serverdensity: {}", &config.disable_serverdensity);

    let mut registry = Registry::default();
    registry.register(
        "udpagent.requests.metrics",
        "requests to /metrics",
        METRIC_COUNTER_REQUESTS.clone(),
    );
    registry.register(
        "udpagent.errors",
        "internal errors",
        METRIC_COUNTER_ERRORS.clone(),
    );
    registry.register(
        "udpagent.udppackets",
        "udp packets",
        METRIC_COUNTER_UDP_PACKETS.clone(),
    );

    let metric_registry = Arc::new(RwLock::new(registry));
    let (sender, receiver) = channel::<InboundMetric>(100_000);

    // server density aggregator
    let server_density_aggregator_handle = if config.disable_serverdensity {
        None
    } else {
        let server_density_config =
            ServerDensityConfig::from_args(matches).context("serverdensity args")?;
        let server_density_aggregator_receiver = sender.subscribe();
        Some(tokio::spawn(async move {
            let server_density_aggregator = ServerDensityAggregator::new(server_density_config);
            server_density_aggregator
                .run(server_density_aggregator_receiver)
                .await;
        }))
    };

    let processor_config = config.clone();
    let processor_registry = metric_registry.clone();
    let processor_receiver = receiver;
    let processor_handle = tokio::spawn(async move {
        let mut processor = Processor::new(processor_config, processor_registry);
        processor.run(processor_receiver).await;
    });

    let udp_server_config = config.clone();
    let udp_server_handle = tokio::spawn(async move {
        let udp_server = UdpServer::new(udp_server_config, sender);
        udp_server.run().await;
    });

    // bind the http server to serve open metrics requests
    let http_server_registry = metric_registry.clone();
    let http_server_handle = http_server::bind(&config, http_server_registry);

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
            _ = async { server_density_aggregator_handle.expect("must be given").await }, if server_density_aggregator_handle.is_some() => {
                eprintln!("Serverdensity aggregator failed");
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
