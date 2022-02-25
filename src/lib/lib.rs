#[derive(Debug, PartialEq)]
pub enum MetricType {
    SUM,
    AVERAGE,
    PEAK,
    MIN
}

impl MetricType {
    pub fn from_u16(v : u16) -> Option<MetricType>
    {
        match v {
            42 => Some(MetricType::SUM),
            43 => Some(MetricType::AVERAGE),
            44 => Some(MetricType::PEAK),
            45 => Some(MetricType::MIN),
            _ => None
        }
    }
}

pub fn create_package() {

    /*
    let metric_type = match MetricType::from_u16(BigEndian::read_u16(&buf[0..2])) {
        Some(m) => m,
        None => {
            return Err("unsupported metric type".to_string());
        }
    };

    let count = BigEndian::read_i32(&buf[2..6]);
    let name = String::from_utf8_lossy(&buf[6..amt]).to_string().replace("\"", "");

     */
}