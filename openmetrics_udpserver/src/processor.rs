use std::collections::hash_map::Entry;
use crate::config::Config;
use openmetrics_udpserver_lib::MetricType;
use prometheus_client::registry::{Metric, Registry};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64};
use std::time::Duration;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use tokio::sync::broadcast::Receiver;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct InboundMetric {
    pub name: String,
    pub count: i32,
    pub metric_type: MetricType,
}

pub struct Processor {
    config: Config,
    counters: HashMap<String, Counter>,
    gauges: HashMap<String, Gauge>,
    metric_registry: Arc<RwLock<Registry>>,
}

impl Processor {
    pub fn new(config: Config, metric_registry: Arc<RwLock<Registry>>) -> Self {
        Processor {
            config,
            counters: HashMap::new(),
            gauges: HashMap::new(),
            metric_registry,
        }
    }

    pub async fn run(&mut self, mut receiver: Receiver<InboundMetric>) {
        loop {
            match receiver.recv().await {
                Ok(inbound_metric) => {
                    if inbound_metric.name.trim().is_empty() {
                        eprintln!("got empty metric name");
                        continue;
                    }

                    if self.config.debug {
                        println!(
                            "got metric [type={:?}, name={}, count={}]",
                            &inbound_metric.metric_type, &inbound_metric.name, &inbound_metric.count
                        );
                    }

                    match inbound_metric.metric_type {
                        MetricType::Min | MetricType::Average | MetricType::Peak => self.handle_gauge(&inbound_metric),
                        MetricType::Sum => self.handle_counter(&inbound_metric),
                    }
                }
                Err(e) => {
                    eprintln!("processor recv error {:#?}, investigate!", e);
                    ::tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        }
    }

    fn handle_counter(&mut self, inbound_metric: &InboundMetric) {
        match self.counters.entry(inbound_metric.name.clone()) {
            Entry::Occupied(mut v) => {
                v.get_mut().inc_by(inbound_metric.count as u64);
            },
            Entry::Vacant(vacant) => {
                let counter = Counter::<u64, AtomicU64>::default();
                counter.inc_by(inbound_metric.count as u64);
                vacant.insert(counter);
            }
        }
    }

    fn handle_gauge(&mut self, inbound_metric: &InboundMetric) {
        match self.gauges.entry(inbound_metric.name.clone()) {
            Entry::Occupied(mut v) => {
                v.get_mut().set(inbound_metric.count as i64);
            },
            Entry::Vacant(vacant) => {
                let mut gauge = Gauge::<i64, AtomicI64>::default();
                gauge.set(inbound_metric.count as i64);
                vacant.insert(gauge);
            }
        }
    }
}
