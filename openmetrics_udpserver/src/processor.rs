use crate::config::Config;
use crate::metrics::counter::CounterMetric;
use crate::metrics::resetting_value_metric::ResettingSingleValMetric;
use crate::metrics::ModifyMetric;
use anyhow::anyhow;
use openmetrics_udpserver_lib::MetricType;
use prometheus_client::registry::{Metric, Registry};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::broadcast::Receiver;
use tokio::sync::RwLock;
use tokio::task::yield_now;

#[derive(Debug, Clone)]
pub struct InboundMetric {
    pub name: String,
    pub count: i32,
    pub metric_type: MetricType,
}

pub struct Processor {
    config: Config,
    metrics: HashMap<String, Arc<dyn ModifyMetric + Send + Sync>>,
    metric_registry: Arc<RwLock<Registry>>,
}

impl Processor {
    pub fn new(config: Config, metric_registry: Arc<RwLock<Registry>>) -> Self {
        Processor {
            config,
            metrics: HashMap::new(),
            metric_registry,
        }
    }

    pub async fn run(&mut self, mut receiver: Receiver<InboundMetric>) {
        let regex_allowed_chars = Regex::new(r"^[^a-zA-Z_:]|[^a-zA-Z0-9_:]")
            .expect("Unable to compile metrics naming regex, should not happen");
        loop {

            match receiver.recv().await {
                Ok(inbound_metric) => {
                    let metric_name = regex_allowed_chars
                        .replace_all(&inbound_metric.name.replace('.', "_"), "")
                        .trim()
                        .to_string();
                    if metric_name.is_empty() {
                        eprintln!("got empty metric name");
                        continue;
                    }

                    if self.config.debug {
                        println!(
                            "got metric [type={:?}, name={}, count={}]",
                            &inbound_metric.metric_type, &metric_name, &inbound_metric.count
                        );
                    }

                    match inbound_metric.metric_type {
                        MetricType::Min | MetricType::Average | MetricType::Peak => {
                            let metric_get_result = self
                                .get_or_register_metric(&metric_name, || {
                                    ResettingSingleValMetric::default()
                                });
                            Self::observe_metric(metric_get_result, inbound_metric).await;
                        }
                        MetricType::Sum => {
                            let metric_get_result = self
                                .get_or_register_metric(&metric_name, || {
                                    CounterMetric::default()
                                });
                            Self::observe_metric(metric_get_result, inbound_metric).await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("processor recv error {:#?}, investigate!", e);
                    ::tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        }
    }

    async fn observe_metric(
        metric_get_result: anyhow::Result<Arc<dyn ModifyMetric + Send + Sync>>,
        inbound_metric: InboundMetric,
    ) {
        match metric_get_result {
            Ok(metric) => metric.observe(inbound_metric.count),
            Err(error) => {
                eprintln!(
                    "Unable to get metric for observation, dropped info from metric {}: {}",
                    inbound_metric.name, error
                );
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    fn get_or_register_metric<M, F: FnOnce() -> M>(
        &mut self,
        metric_name: &String,
        metric_type_factory: F,
    ) -> anyhow::Result<Arc<dyn ModifyMetric + Send + Sync>>
    where
        M: Metric + ModifyMetric + Clone + Send + Sync,
    {
        match self.metrics.get(metric_name) {
            Some(metric) => Ok(metric.clone()),
            None => {
                return match self.metric_registry.try_write() {
                    Ok(mut registry) => {
                        let metric = metric_type_factory();
                        (*registry).register(
                            metric_name,
                            format!("The {} metric", metric_name),
                            metric.clone(),
                        );

                        let rc_metric = Arc::new(metric);
                        self.metrics.insert(metric_name.clone(), rc_metric.clone());
                        Ok(rc_metric)
                    }
                    Err(_) => Err(anyhow!("unable to lock metric registry for writing")),
                };
            }
        }
    }
}
