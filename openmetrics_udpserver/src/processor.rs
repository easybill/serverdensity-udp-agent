use crate::config::Config;
use crate::metrics::resetting_counter::ResettingCounterMetric;
use crate::metrics::resetting_value_metric::ResettingSingleValMetric;
use crate::metrics::ModifyMetric;
use anyhow::anyhow;
use crossbeam_channel::Receiver;
use openmetrics_udpserver_lib::MetricType;
use prometheus_client::registry::{Metric, Registry};
use regex::Regex;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct InboundMetric {
    pub name: String,
    pub count: i32,
    pub metric_type: MetricType,
}

pub struct Processor {
    config: Config,
    metrics: HashMap<String, Rc<dyn ModifyMetric>>,
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

    pub fn run(&mut self, receiver: Receiver<InboundMetric>) -> anyhow::Result<(), anyhow::Error> {
        let regex_allowed_chars = Regex::new(r"[^0-9a-zA-ZäöüÄÖÜß\-()._]*")?;
        loop {
            match receiver.recv() {
                Ok(inbound_metric) => {
                    let metric_name = regex_allowed_chars
                        .replace_all(&inbound_metric.name, "")
                        .trim()
                        .to_string();
                    if metric_name.is_empty() {
                        eprintln!("got empty metric name");
                        continue;
                    }

                    if self.config.debug {
                        println!(
                            "got metric [type={:?}, name={}, count={}]",
                            &inbound_metric.metric_type,
                            &inbound_metric.name,
                            &inbound_metric.count
                        );
                    }

                    match inbound_metric.metric_type {
                        MetricType::Min | MetricType::Average | MetricType::Peak => {
                            let metric = self
                                .get_or_register_metric(&inbound_metric.name, || {
                                    ResettingSingleValMetric::default()
                                })
                                .unwrap();
                            metric.observe(inbound_metric.count);
                        }
                        MetricType::Sum => {
                            let metric = self
                                .get_or_register_metric(&inbound_metric.name, || {
                                    ResettingCounterMetric::default()
                                })
                                .unwrap();
                            metric.observe(inbound_metric.count);
                        }
                    }
                }
                Err(_) => return Err(anyhow!("metric sender disconnected")),
            }
        }
    }

    fn get_or_register_metric<M, F: FnOnce() -> M>(
        &mut self,
        metric_name: &String,
        metric_type_factory: F,
    ) -> anyhow::Result<Rc<dyn ModifyMetric>>
    where
        M: Metric + ModifyMetric + Clone,
    {
        match self.metrics.get(metric_name) {
            Some(metric) => Ok(metric.clone()),
            None => {
                return match self.metric_registry.try_write() {
                    Ok(mut registry) => {
                        let metric = metric_type_factory();
                        (*registry).register(metric_name, "", metric.clone());

                        let rc_metric = Rc::new(metric);
                        self.metrics.insert(metric_name.clone(), rc_metric.clone());
                        Ok(rc_metric as Rc<dyn ModifyMetric>)
                    }
                    Err(_) => Err(anyhow!("unable to lock metric registry for writing")),
                };
            }
        }
    }
}
