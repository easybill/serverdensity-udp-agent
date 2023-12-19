use crate::processor::InboundMetric;
use crate::serverdensity::{AverageHandler, MinHandler, PeakHandler, SumHandler};
use clap::ArgMatches;
use openmetrics_udpserver_lib::MetricType;
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::time::{Duration, SystemTime};
use anyhow::{anyhow, Context};
use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::broadcast::Receiver;
use crate::METRIC_COUNTER_ERRORS;

#[derive(Clone, Debug)]
pub struct ServerDensityConfig {
    pub token: String,
    pub account_url: String,
    pub agent_key: String,
    pub serverdensity_endpoint: String,
}

impl ServerDensityConfig {
    pub fn from_args(matches: ArgMatches) -> Result<Self, ::anyhow::Error> {
        let token_option = matches.get_one::<String>("token");

        let mut base_config = ServerDensityConfig {
            token: token_option
                .context("'--token' has to be provided, if server density is not disabled with '--disable-serverdensity'")?
                .to_string(),
            account_url: matches
                .get_one::<String>("account-url")
                .unwrap_or(&"".to_string())
                .to_string(),
            agent_key: matches
                .get_one::<String>("agent-key")
                .unwrap_or(&"".to_string())
                .to_string(),
            serverdensity_endpoint: matches
                .get_one::<String>("serverdensity-endpoint")
                .context("'--serverdensity-endpoint' has to be provided, if server density is not disabled with '--disable-serverdensity'")?
                .to_string(),
        };

        if matches.get_one::<String>("config").is_some() {
            let config_file = matches.get_one::<String>("config").unwrap().to_string();
            match base_config.apply_config_file(&config_file) {
                Ok(_) => println!("successfully read config_file: {}", &config_file),
                Err(_) => {
                    return Err(anyhow!("could not read config_file: {}", &config_file));
                }
            };
        }

        if base_config.agent_key.trim() == "" || base_config.account_url.trim() == "" {
            return Err(anyhow!("agent-key or account-url not given."));
        }

        Ok(base_config)
    }

    fn line_value(&self, line: &str) -> String {
        let value = line
            .trim()
            .split(':')
            .map(|x| x.trim().to_string())
            .collect::<Vec<String>>();

        if value.len() != 2 {
            return "".to_string();
        }

        value[1].clone()
    }

    pub fn apply_config_file(&mut self, filename: &str) -> Result<(), String> {
        let file = File::open(filename).map_err(|_| "could not open config file".to_string())?;
        let mut buf_reader = BufReader::new(file);

        let mut content = String::new();
        buf_reader
            .read_to_string(&mut content)
            .map_err(|_| "could not read config file".to_string())?;

        for line in content.split('\n') {
            if line.trim().starts_with('#') || line.trim().starts_with('[') {
                continue;
            }

            if line.trim().starts_with("agent_key") {
                self.agent_key = self.line_value(line);
                continue;
            }

            if line.trim().starts_with("sd_account") {
                self.account_url = self.line_value(line);
                continue;
            }
        }

        Ok(())
    }
}

pub struct ServerDensityAggregator {
    config: ServerDensityConfig,
    http_client: Client,
    api_postback_uri: String,
}

impl ServerDensityAggregator {
    pub fn new(config: ServerDensityConfig) -> ServerDensityAggregator {
        ServerDensityAggregator {
            config: config.clone(),
            http_client: Client::new(),
            api_postback_uri: format!(
                "{}/alerts/postbacks?token={}",
                &config.serverdensity_endpoint, &config.token
            ),
        }
    }

    pub async fn run(&self, mut receiver: Receiver<InboundMetric>) {
        let regex = Regex::new(r"[^0-9a-zA-ZäöüÄÖÜß\-()._]*").expect("failed to compile regex");

        let mut metricmap = HashMap::new();

        let handler_sum = SumHandler::new();
        let mut handler_avg = AverageHandler::new();
        let handler_peak = PeakHandler::new();
        let handler_min = MinHandler::new();
        let mut flush_interval = ::tokio::time::interval(Duration::from_secs(10));

        loop {

            ::tokio::select! {
                _ = flush_interval.tick() => {
                    handler_sum.flush(&mut metricmap);
                    handler_avg.flush(&mut metricmap);
                    handler_peak.flush(&mut metricmap);
                    handler_min.flush(&mut metricmap);
                    self.push_to_serverdensity(&mut metricmap).await;
                },
                msg = receiver.recv() => {
                    match msg {
                        Ok(metric) => {
                            let metric_name = regex.replace_all(&metric.name, "").trim().to_string();

                            if metric_name.is_empty() {
                                println!("got empty metric name.");
                                continue;
                            }

                            match metric.metric_type {
                                MetricType::Sum => {
                                    handler_sum.handle(&metric_name, &metric, &mut metricmap);
                                }
                                MetricType::Average => {
                                    handler_avg.handle(&metric_name, &metric, &mut metricmap);
                                }
                                MetricType::Peak => {
                                    handler_peak.handle(&metric_name, &metric, &mut metricmap);
                                }
                                MetricType::Min => {
                                    handler_min.handle(&metric_name, &metric, &mut metricmap);
                                }
                            };
                        }
                        Err(e) => {
                            METRIC_COUNTER_ERRORS.inc();
                            eprintln!("aggregator recv error {:#?}, investigate!", e);
                            ::tokio::time::sleep(Duration::from_millis(300)).await;
                        }
                    };
                }
            };
        }
    }

    pub fn create_plugin_map(map: &HashMap<String, i32>) -> String {
        let mut outermap: HashMap<String, HashMap<String, i32>> = HashMap::new();

        for (k, v) in map {
            let len = k.len();

            match k.find('.') {
                Some(index) if index > 0 && index + 1 != len => {
                    outermap
                        .entry(k[..index].to_string())
                        .or_default()
                        .insert(k[index + 1..].to_string(), *v);
                }
                _ if k.trim() != "" => {
                    outermap
                        .entry("custom".to_string())
                        .or_default()
                        .insert(k.to_string(), *v);
                }
                _ => {}
            };
        }

        outermap
            .iter()
            .map(|(k, v)| {
                let mut buf = String::new();
                buf.push('"');
                buf.push_str(k);
                buf.push_str("\":{");

                buf.push_str(
                    &v.iter()
                        .map(|(k, v)| format!("\"{}\":\"{}\"", k, v))
                        .collect::<Vec<String>>()
                        .join(",")
                        .to_string(),
                );

                buf.push('}');
                buf
            })
            .collect::<Vec<String>>()
            .join(",")
            .to_string()
    }

    pub async fn push_to_serverdensity(&self, metricmap: &mut HashMap<String, i32>) {
        if metricmap.is_empty() {
            return;
        }

        let mut payload = "{\"agentKey\":\"".to_string();
        payload.push_str(&self.config.agent_key);
        payload.push_str("\",\"plugins\":{");
        payload.push_str(&Self::create_plugin_map(metricmap));
        payload.push_str("}}");

        *metricmap = HashMap::new();

        let send_data_to_backend_time = SystemTime::now();

        let data = &[
            ("payload", &payload),
            ("hash", &format!("{:x}", md5::compute(&payload))),
        ];

        println!("Data to send to ServerDensity Backend {:#?}", &data);

        let res = self
            .http_client
            .post(&self.api_postback_uri)
            .header("X-Forwarded-Host", self.config.account_url.clone())
            .form(data)
            .timeout(Duration::from_secs(30))
            .send()
            .await;

        let send_data_to_backend_tooked_in_ms = match send_data_to_backend_time.elapsed() {
            Ok(duration) => {
                (duration.as_secs() * 1000) + (duration.subsec_nanos() as u64 / 1000000)
            }
            Err(_) => {
                METRIC_COUNTER_ERRORS.inc();
                println!("seems to have trouble with the clock, should never happen.");
                return;
            }
        };

        match res {
            Ok(r) => {
                let response_status = r.status();
                match r.text().await {
                    Ok(content) => {
                        println!("submitted to serverdensity, took {}ms \n--- metrics --- \n{:#?} \n\n{} \n----\n", &send_data_to_backend_tooked_in_ms, data, &content);
                    }
                    Err(err) => {
                        METRIC_COUNTER_ERRORS.inc();
                        println!("submitted to serverdentity, status: {}, but could not read response: {}", response_status, err);
                    }
                }
            }
            Err(err) => {
                METRIC_COUNTER_ERRORS.inc();
                println!("failed to send to serverdensity, status {:?}", err.status());
                println!("error: {:?}", err);
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::serverdensity::aggregator::ServerDensityAggregator;
    use ::std::collections::HashMap;

    #[test]
    fn it_works() {
        let mut m = HashMap::new();
        m.insert("foo".to_string(), 2);
        m.insert("foo.".to_string(), 3);
        m.insert(".foo.bar.barr".to_string(), 4);

        m.insert("foo.bar".to_string(), 5);
        m.insert("foo.bar.baz".to_string(), 6);

        let out = ServerDensityAggregator::create_plugin_map(&m);

        println!("{}\n", &format!("{}", out));

        let mut m = HashMap::new();
        m.insert("foo".to_string(), 2);
        let out = ServerDensityAggregator::create_plugin_map(&m);
        println!("{}\n", &format!("{}", out));

        let mut m = HashMap::new();
        m.insert("foo.bar".to_string(), 2);
        let out = ServerDensityAggregator::create_plugin_map(&m);
        println!("{}\n", &format!("{}", out));

        let mut m = HashMap::new();
        let out = ServerDensityAggregator::create_plugin_map(&m);
        println!("{}\n", &format!("{}", out));

        /*
        the assert doesnt work because the map is not ordered.

        assert_eq!(
            r#""foo":{"bar":"5","bar.baz":"6"},"custom":{".foo.bar.barr":"4","foo":"2","foo.":"3"}"#,
            &format!("{}", out)
        );
        */
    }
}
