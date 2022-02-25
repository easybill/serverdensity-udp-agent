use std::net::UdpSocket;
use byteorder::{BigEndian};
use byteorder::ByteOrder;
use crate::config::Config;
use crate::Metric;
use std::sync::mpsc::Sender;


pub struct UdpServer;

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

impl UdpServer {
    pub fn new() -> Self {
        UdpServer {}
    }

    pub fn run(&self, config: &Config, sender: Sender<Metric>)
    {
            let mut socket = match UdpSocket::bind(&config.bind) {
            Ok(s) => s,
            Err(_) => {
                println!("could not listen, may someone is already listen on this port or the address is invalid?");
                return;
            }
        };

        loop {
            
            match self.read(&mut socket) {
                Err(_) => {
                    println!("could not read from socket.\n");
                    continue;
                },
                Ok(m) => {
                    match sender.send(m) {
                        Ok(_) => {},
                        Err(_) => {
                            println!("could not recv. metric");
                        }
                    }
                }
            }
        }
    }

    fn read(&self, socket : &mut UdpSocket) -> Result<Metric, String>
    {
        let mut buf = [0; 300];
        let (amt, _) = socket.recv_from(&mut buf).or_else(|_|Err("Could recv from Socket.".to_string()))?;

        if amt <= 6 {
            return Err("UDP Package size is to small.".to_string());
        }

        let metric_type = match MetricType::from_u16(BigEndian::read_u16(&buf[0..2])) {
            Some(m) => m,
            None => {
                return Err("unsupported metric type".to_string());
            }
        };

        let count = BigEndian::read_i32(&buf[2..6]);
        let name = String::from_utf8_lossy(&buf[6..amt]).to_string().replace("\"", "");

        Ok(Metric {
            count: count,
            name,
            metric_type
        })
    }
}