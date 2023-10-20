use crate::config::Config;
use crate::processor::InboundMetric;
use byteorder::BigEndian;
use byteorder::ByteOrder;
use crossbeam_channel::Sender;
use openmetrics_udpserver_lib::MetricType;
use std::net::UdpSocket;

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

    pub fn run(&self) -> anyhow::Result<()> {
        let mut udp_socket = UdpSocket::bind(&self.config.udp_bind)?;
        loop {
            match self.read(&mut udp_socket) {
                Ok(metric) => {
                    if let Err(err) = self.metric_sender.send(metric) {
                        eprintln!("Unable to process inbound metric: {}", err);
                    }
                }
                Err(err) => {
                    eprintln!("could not read message from socket: {}", err);
                }
            }
        }
    }

    fn read(&self, socket: &mut UdpSocket) -> Result<InboundMetric, String> {
        let mut buf = [0; 300];
        let (amt, _) = socket
            .recv_from(&mut buf)
            .map_err(|_| "Couldn't recv from socket".to_string())?;

        if amt <= 6 {
            return Err("UDP Package size is to small".to_string());
        }

        let metric_type = match MetricType::from_u16(BigEndian::read_u16(&buf[0..2])) {
            Some(m) => m,
            None => {
                return Err("unsupported metric type".to_string());
            }
        };

        let count = BigEndian::read_i32(&buf[2..6]);
        let name = String::from_utf8_lossy(&buf[6..amt])
            .to_string()
            .replace('"', "");

        Ok(InboundMetric {
            count,
            name,
            metric_type,
        })
    }
}
