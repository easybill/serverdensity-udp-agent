pub mod counter;
pub mod gauge;

pub trait ModifyMetric {
    fn observe(&self, value: i32);
}
