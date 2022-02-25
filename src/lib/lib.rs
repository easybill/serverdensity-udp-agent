use bytes::{BufMut, BytesMut};

#[derive(Debug, PartialEq)]
pub enum MetricType {
    SUM,
    AVERAGE,
    PEAK,
    MIN,
}

impl MetricType {
    pub fn from_u16(v: u16) -> Option<MetricType>
    {
        match v {
            42 => Some(MetricType::SUM),
            43 => Some(MetricType::AVERAGE),
            44 => Some(MetricType::PEAK),
            45 => Some(MetricType::MIN),
            _ => None
        }
    }

    pub fn to_u16(&self) -> u16 {
        match self {
            Self::SUM => 42,
            Self::AVERAGE => 43,
            Self::PEAK => 44,
            Self::MIN => 45
        }
    }
}

pub fn create_package<S>(metric_type: MetricType, name: S, count: i32) -> Result<Vec<u8>, &'static str>
    where S: AsRef<str> {
    let mut buf = BytesMut::new();
    buf.put_u16(metric_type.to_u16());
    buf.put_i32(count);
    buf.put_slice(name.as_ref().as_bytes());

    if buf.len() > 300 {
        return Err("could not create a package that is larger than 300 bytes!");
    }

    Ok(buf.to_vec())
}

pub fn create_package_sum<S>(name: S, count: i32) -> Result<Vec<u8>, &'static str>
    where S: AsRef<str> {
    create_package(MetricType::SUM, name, count)
}

pub fn create_package_min<S>(name: S, count: i32) -> Result<Vec<u8>, &'static str>
    where S: AsRef<str> {
    create_package(MetricType::MIN, name, count)
}

pub fn create_package_peak<S>(name: S, count: i32) -> Result<Vec<u8>, &'static str>
    where S: AsRef<str> {
    create_package(MetricType::PEAK, name, count)
}

pub fn create_package_average<S>(name: S, count: i32) -> Result<Vec<u8>, &'static str>
    where S: AsRef<str> {
    create_package(MetricType::AVERAGE, name, count)
}