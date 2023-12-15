use fnv::FnvHashMap;
use crate::processor::ProcessorMetric;

pub struct AggragatorPeakGauge {
    buffer: FnvHashMap<String, u64>,
}

impl AggragatorPeakGauge {
    pub fn new() -> Self {
        Self {
            buffer: FnvHashMap::default(),
        }
    }

    pub fn handle(&mut self, metric: &ProcessorMetric) {
        let e: &mut u64 = self.buffer.entry(metric.name.clone()).or_insert(0);

        if *e < metric.count {
            *e = metric.count;
        }
    }

    pub fn reset_and_fetch(&mut self) -> FnvHashMap<String, u64> {
        let mut swap_map = FnvHashMap::default();
        ::std::mem::swap(&mut swap_map, &mut self.buffer);
        swap_map
    }
}