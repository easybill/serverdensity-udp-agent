use crate::aggregator::average::AggragatorAverageGauge;
use crate::aggregator::min::AggragatorMinGauge;
use crate::aggregator::peak::AggragatorPeakGauge;
use crate::config::Config;
use crate::METRIC_COUNTER_ERRORS;
use openmetrics_udpserver_lib::MetricType;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use regex::Regex;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicI64, AtomicU64};
use std::sync::Arc;
use std::time::Duration;
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
    counters: ::fnv::FnvHashMap<String, Counter>,
    gauges: ::fnv::FnvHashMap<String, Gauge>,
    aggregator_peak_gauge: AggragatorPeakGauge,
    aggregator_min_gauge: AggragatorMinGauge,
    aggregator_average_gauge: AggragatorAverageGauge,
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
            aggregator_min_gauge: AggragatorMinGauge::new(),
            aggregator_average_gauge: AggragatorAverageGauge::new(),
            metric_registry,
        }
    }

    pub async fn run(&mut self, mut receiver: Receiver<InboundMetric>) {
        let regex_allowed_chars = Regex::new(r"^[^a-zA-Z_:]|[^a-zA-Z0-9_:]")
            .expect("Unable to compile metrics naming regex, should not happen");

        let mut aggregation_interval = ::tokio::time::interval(Duration::from_secs(30));

        loop {
            ::tokio::select! {
                _ = aggregation_interval.tick() => {
                    self.handle_aggragation_flush().await
                },
                msg = receiver.recv() => {
                    match msg {
                        Ok(inbound_metric) => self.handle_metric(&regex_allowed_chars, inbound_metric).await,
                        Err(e) => {
                            METRIC_COUNTER_ERRORS.inc();
                            eprintln!("processor recv error {:#?}, investigate!", e);
                            ::tokio::time::sleep(Duration::from_millis(300)).await;
                        }
                    }
                }
            }
        }
    }

    async fn handle_aggragation_flush(&mut self) {
        for (k, v) in self.aggregator_average_gauge.reset_and_fetch().into_iter() {
            self.handle_gauge(k, v).await
        }

        for (k, v) in self.aggregator_min_gauge.reset_and_fetch().into_iter() {
            self.handle_gauge(k, v).await
        }

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
            MetricType::Min => self.aggregator_min_gauge.handle(&processor_metric),
            MetricType::Average => self.aggregator_average_gauge.handle(&processor_metric),
            MetricType::Sum => self.handle_counter(&processor_metric).await,
        }
    }

    async fn handle_counter(&mut self, metric: &ProcessorMetric) {
        match self.counters.entry(metric.name.clone()) {
            Entry::Occupied(mut v) => {
                v.get_mut().inc_by(metric.count);
            }
            Entry::Vacant(vacant) => {
                let counter = Counter::<u64, AtomicU64>::default();
                counter.inc_by(metric.count);
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
            }
            Entry::Vacant(vacant) => {
                let gauge = Gauge::<i64, AtomicI64>::default();
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
