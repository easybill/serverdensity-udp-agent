use bytes::{BufMut, BytesMut};
use thiserror::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MetricType {
    Sum,
    Average,
    Peak,
    Min,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Error)]
pub enum EncodeError {
    #[error("buffer size must be smaller than 300 bytes, got {0} bytes")]
    BufferTooLarge(usize),
}

impl MetricType {
    pub fn from_u16(v: u16) -> Option<MetricType> {
        match v {
            42 => Some(MetricType::Sum),
            43 => Some(MetricType::Average),
            44 => Some(MetricType::Peak),
            45 => Some(MetricType::Min),
            _ => None,
        }
    }

    pub fn to_u16(&self) -> u16 {
        match self {
            Self::Sum => 42,
            Self::Average => 43,
            Self::Peak => 44,
            Self::Min => 45,
        }
    }
}

pub fn create_package<S>(
    metric_type: MetricType,
    name: S,
    count: i32,
) -> Result<Vec<u8>, EncodeError>
where
    S: AsRef<str>,
{
    let mut buf = BytesMut::new();
    buf.put_u16(metric_type.to_u16());
    buf.put_i32(count);
    buf.put_slice(name.as_ref().as_bytes());

    if buf.len() > 300 {
        return Err(EncodeError::BufferTooLarge(buf.len()));
    }

    Ok(buf.to_vec())
}

pub fn create_package_sum<S>(name: S, count: i32) -> Result<Vec<u8>, EncodeError>
where
    S: AsRef<str>,
{
    create_package(MetricType::Sum, name, count)
}

pub fn create_package_min<S>(name: S, count: i32) -> Result<Vec<u8>, EncodeError>
where
    S: AsRef<str>,
{
    create_package(MetricType::Min, name, count)
}

pub fn create_package_peak<S>(name: S, count: i32) -> Result<Vec<u8>, EncodeError>
where
    S: AsRef<str>,
{
    create_package(MetricType::Peak, name, count)
}

pub fn create_package_average<S>(name: S, count: i32) -> Result<Vec<u8>, EncodeError>
where
    S: AsRef<str>,
{
    create_package(MetricType::Average, name, count)
}
