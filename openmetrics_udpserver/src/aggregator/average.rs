use fnv::FnvHashMap;
use crate::processor::ProcessorMetric;

pub struct AverageBucket {
    pub sum: u64,
    pub count: u64,
}

pub struct AggragatorAverageGauge {
    buffer: FnvHashMap<String, AverageBucket>,
}

impl AggragatorAverageGauge {
    pub fn new() -> Self {
        Self {
            buffer: FnvHashMap::default(),
        }
    }

    pub fn handle(&mut self, metric: &ProcessorMetric) {
        let bucket: &mut AverageBucket = self
            .buffer
            .entry(metric.name.clone())
            .or_insert(AverageBucket {sum: 0, count: 0});

        bucket.sum += metric.count as u64;
        bucket.count += 1;
    }

    pub fn reset_and_fetch(&mut self) -> FnvHashMap<String, u64> {
        let mut buf = FnvHashMap::default();
        for (k, v) in &self.buffer {
            buf.insert(k.to_string(), (v.sum / v.count) as u64);
        }

        buf
    }
}