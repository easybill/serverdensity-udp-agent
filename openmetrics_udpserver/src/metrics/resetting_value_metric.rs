use std::fmt::Error;
use std::sync::atomic::Ordering;
use atomic_float::AtomicF64;
use std::sync::Arc;

use crate::metrics::ModifyMetric;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::{MetricType, TypedMetric};

#[derive(Debug, Default, Clone)]
pub struct ResettingSingleValMetric {
    val: Arc<AtomicF64>,
}

impl ModifyMetric for ResettingSingleValMetric {
    fn observe(&self, value: i32) {
        if let Ok(val_as_f64) = f64::try_from(value) {
            self.val.store(val_as_f64, Ordering::Relaxed);
        }
    }
}

impl EncodeMetric for ResettingSingleValMetric {
    fn encode(&self, mut encoder: MetricEncoder) -> Result<(), Error> {
        let current_value = self.val.load(Ordering::Relaxed);
        encoder.encode_gauge::<f64>(&current_value)
    }

    fn metric_type(&self) -> MetricType {
        Self::TYPE
    }
}

impl TypedMetric for ResettingSingleValMetric {
    const TYPE: MetricType = MetricType::Gauge;
}
