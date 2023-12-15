use std::collections::hash_map::Entry;
use crate::config::Config;
use openmetrics_udpserver_lib::MetricType;
use prometheus_client::registry::{Metric, Registry};
use regex::Regex;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64};
use std::time::Duration;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use tokio::sync::broadcast::Receiver;
use tokio::sync::RwLock;
use crate::aggragator::AggragatorPeakGauge;

#[derive(Debug, Clone)]
pub struct InboundMetric {
    pub name: String,
    pub count: i32,
    pub metric_type: MetricType,
}

pub struct Processor {
    config: Config,
    counters: ::fnv::FnvHashMap<String, Counter>,
    gauges: ::fnv::FnvHashMap<String, Gauge>,
    aggregator_peak_gauge: AggragatorPeakGauge,
    metric_registry: Arc<RwLock<Registry>>,
}

pub struct ProcessorMetric {
    pub name: String,
    pub count: u64,
    pub metric_type: MetricType,
}

impl ProcessorMetric {
    pub fn from_inbound(name: String, inbound_metric: InboundMetric) -> Self {

        let name = match inbound_metric.metric_type {
            // this is some kind of legacy. we would end up with _total_total because the application is already sending _total and the client is also appending _total
            MetricType::Sum => name.trim_end_matches("_total").to_string(),
            _ => name,
        };

        Self {
            name,
            count: inbound_metric.count as u64,
            metric_type: inbound_metric.metric_type,
        }
    }
}

impl Processor {
    pub fn new(config: Config, metric_registry: Arc<RwLock<Registry>>) -> Self {
        Processor {
            config,
            counters: ::fnv::FnvHashMap::default(),
            gauges: ::fnv::FnvHashMap::default(),
            aggregator_peak_gauge: AggragatorPeakGauge::new(),
            metric_registry,
        }
    }

    pub async fn run(&mut self, mut receiver: Receiver<InboundMetric>) {
        let regex_allowed_chars = Regex::new(r"^[^a-zA-Z_:]|[^a-zA-Z0-9_:]")
            .expect("Unable to compile metrics naming regex, should not happen");

        let mut aggregation_interval = ::tokio::time::interval(Duration::from_secs(60));

        loop {
            ::tokio::select! {
                _ = aggregation_interval.tick() => {
                    self.handle_aggragation_flush().await
                },
                msg = receiver.recv() => {
                    match msg {
                        Ok(inbound_metric) => self.handle_metric(&regex_allowed_chars, inbound_metric).await,
                        Err(e) => {
                            eprintln!("processor recv error {:#?}, investigate!", e);
                            ::tokio::time::sleep(Duration::from_millis(300)).await;
                        }
                    }
                }
            }
        }
    }

    async fn handle_aggragation_flush(&mut self) {
        for (k, v) in self.aggregator_peak_gauge.reset_and_fetch().into_iter() {
            self.handle_gauge(k, v).await
        }
    }

    async fn handle_metric(&mut self, regex_allowed_chars: &Regex, inbound_metric: InboundMetric) {
        let metric_name = regex_allowed_chars
            .replace_all(&inbound_metric.name.replace('.', "_"), "")
            .trim()
            .to_string();

        if metric_name.is_empty() {
            eprintln!("got empty metric name");
            return;
        }

        let processor_metric = ProcessorMetric::from_inbound(metric_name, inbound_metric);

        if self.config.debug {
            println!(
                "got metric [type={:?}, name={}, count={}]",
                &processor_metric.metric_type, &processor_metric.name, &processor_metric.count
            );
        }

        match processor_metric.metric_type {
            MetricType::Peak => self.aggregator_peak_gauge.handle(&processor_metric),
            MetricType::Min | MetricType::Average => self.handle_gauge(&processor_metric).await,
            MetricType::Sum => self.handle_counter(&processor_metric).await,
        }
    }

    async fn handle_counter(&mut self, metric: &ProcessorMetric) {
        match self.counters.entry(metric.name.clone()) {
            Entry::Occupied(mut v) => {
                v.get_mut().inc_by(metric.count as u64);
            },
            Entry::Vacant(vacant) => {
                let counter = Counter::<u64, AtomicU64>::default();
                counter.inc_by(metric.count as u64);
                vacant.insert(counter.clone());

                {
                    let mut registry = self.metric_registry.write().await;
                    registry.register(metric.name.clone(), metric.name.clone(), counter)
                }
            }
        }
    }

    async fn handle_gauge(&mut self, metric_name: String, metric_count: u64) {
        match self.gauges.entry(metric_name.clone()) {
            Entry::Occupied(mut v) => {
                v.get_mut().set(metric_count as i64);
            },
            Entry::Vacant(vacant) => {
                let mut gauge = Gauge::<i64, AtomicI64>::default();
                gauge.set(metric_count as i64);
                vacant.insert(gauge.clone());

                {
                    let mut registry = self.metric_registry.write().await;
                    registry.register(metric_name.clone(), metric_name.clone(), gauge)
                }
            }
        }
    }
}
