use crate::metrics::ModifyMetric;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::{MetricType, TypedMetric};
use std::fmt::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default, Clone)]
pub struct CounterMetric {
    val: Arc<AtomicU64>,
}

impl ModifyMetric for CounterMetric {
    fn observe(&self, value: i32) {
        if let Ok(val_as_u64) = u64::try_from(value) {
            self.val.fetch_add(val_as_u64, Ordering::Relaxed);
        }
    }
}

impl EncodeMetric for CounterMetric {
    fn encode(&self, mut encoder: MetricEncoder) -> Result<(), Error> {
        let current_value = self.val.load(Ordering::Relaxed);
        encoder.encode_counter::<(), u64, u64>(&current_value, None)
    }

    fn metric_type(&self) -> MetricType {
        Self::TYPE
    }
}

impl TypedMetric for CounterMetric {
    const TYPE: MetricType = MetricType::Counter;
}
