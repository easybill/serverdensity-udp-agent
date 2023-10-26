pub mod resetting_counter;
pub mod resetting_value_metric;

pub trait ModifyMetric {
    fn observe(&self, value: i32);
}