use crate::config::Config;
use crate::processor::InboundMetric;
use byteorder::BigEndian;
use byteorder::ByteOrder;
use openmetrics_udpserver_lib::MetricType;
use tokio::net::UdpSocket;
use tokio::sync::broadcast::Sender;

pub struct UdpServer {
    config: Config,
    metric_sender: Sender<InboundMetric>,
}

impl UdpServer {
    pub fn new(config: Config, metric_sender: Sender<InboundMetric>) -> Self {
        UdpServer {
            config,
            metric_sender,
        }
    }

    pub async fn run(&self) {
        let udp_socket = UdpSocket::bind(&self.config.udp_bind)
            .await
            .expect("Unable to bind UDP Server");
        loop {
            let mut buf = [0; 300];
            if let Ok(read_bytes) = udp_socket.recv(&mut buf).await {
                match self.decode_buffer(&buf, read_bytes) {
                    Ok(inbound_metric) => {
                        if let Err(err) = self.metric_sender.send(inbound_metric) {
                            eprintln!("Unable to process inbound metric: {}", err);
                        }
                    }
                    Err(err) => {
                        // it could be, that we are so fast that we read a part of the message, may we need to improve this code.
                        eprintln!("could not decode message from socket: {}", err);
                    }
                }
            }
        }
    }

    fn decode_buffer(&self, data: &[u8], read_bytes: usize) -> Result<InboundMetric, String> {
        let metric_type = match MetricType::from_u16(BigEndian::read_u16(&data[0..2])) {
            Some(m) => m,
            None => return Err("Got unsupported metric type".to_string()),
        };

        let count = BigEndian::read_i32(&data[2..6]);
        let name = String::from_utf8_lossy(&data[6..read_bytes])
            .to_string()
            .replace('"', "");

        Ok(InboundMetric {
            count,
            name,
            metric_type,
        })
    }
}
