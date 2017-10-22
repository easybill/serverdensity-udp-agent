use ::Metric;
use std::collections::HashMap;

pub struct SumHandler;

impl SumHandler {
    pub fn new() -> SumHandler {
        SumHandler {}
    }

    pub fn handle(&self, metric_name : &str, metric : &Metric, metricmap: &mut HashMap<String, i32>) {
        *metricmap.entry(metric_name.to_string()).or_insert(0) += metric.count;
    }

    pub fn flush(&self, _: &mut HashMap<String, i32>) {

    }

}

pub struct AverageBucket {
    sum : u64,
    count: u64
}

impl AverageBucket {
    fn new() -> AverageBucket {
        AverageBucket { sum: 0, count: 0}
    }
}

pub struct AverageHandler {
    buffer: HashMap<String, AverageBucket>
}

impl AverageHandler {
    pub fn new() -> AverageHandler {
        AverageHandler {
            buffer: HashMap::new()
        }
    }

    pub fn handle(&mut self, metric_name : &str, metric : &Metric, _: &mut HashMap<String, i32>) {
        let bucket : &mut AverageBucket = self.buffer.entry(metric_name.to_string()).or_insert(AverageBucket::new());
        bucket.sum += metric.count as u64;
        bucket.count += 1;
    }

    pub fn flush(&mut self, metricmap: &mut HashMap<String, i32>) {
        for (k, v) in &self.buffer {
            metricmap.insert(k.to_string(), (v.sum / v.count) as i32);
        }

        self.buffer = HashMap::new();
    }

}