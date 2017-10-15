extern crate byteorder;
extern crate clap;
extern crate url;
extern crate md5;
extern crate reqwest;

use clap::{Arg, App};
use std::net::UdpSocket;
use byteorder::{BigEndian};
use byteorder::ByteOrder;
use std::sync::mpsc::channel;
use std::thread;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::io::Read;

#[derive(Debug)]
struct Metric {
    pub name: String,
    pub count: i32
}

struct Config {
    pub token: String,
    pub account_url: String
}

fn main() {

    let matches = App::new("Server Density UDP Monitor")
        .version("1.0")
        .author("Tim Glabisch. <serverdensity@tim.ainfach.de>")
        .about("UDP Sender for Serverdendity")
        .arg(Arg::with_name("token")
            .help("Server Density API Token")
            .long("token")
            .required(true)
            .takes_value(true)
            .index(1))
        .arg(Arg::with_name("account-url")
            .help("Set this to your Server Density account url, e.g. example.serverdensity.io")
            .long("account-url")
            .required(true)
            .takes_value(true)
            .index(2))
        .get_matches();

    let config = Config {
        token: matches.value_of("token").unwrap().to_string(),
        account_url: matches.value_of("account-url").unwrap().to_string()
    };

    let (sender, receiver) = channel::<Metric>();

    thread::spawn(move|| {

        let mut metricmap = HashMap::new();

        let mut sys_time = SystemTime::now();

        let client = reqwest::Client::new();

        let uri = &format!("http://127.0.0.1:1337/alerts/postbacks?token={}", &config.token);

        loop {

            match receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(metric) => {
                    *metricmap.entry(metric.name).or_insert(0) += metric.count;
                },
                Err(_) => { }
            };

            
            if sys_time.elapsed().unwrap().as_secs() >= 5 {

                sys_time = SystemTime::now();

                if metricmap.len() == 0 {
                    continue;
                }

                let x = metricmap.iter().map(|(k, v)| {
                    format!("\"{}\":\"{}\"", k, v)
                })
                .collect::<Vec<String>>()
                .join(",")
                .to_string();
                
                let mut payload = "{\"agentKey\":\"23ddab267dff7cde05dc20a28e93c272\",\"plugins\":{\"custom\":{".to_string();

                payload.push_str(&x);
                payload.push_str("}}");

                metricmap = HashMap::new();

                let res = client.post(uri)
                .form(&[
                    ("payload", &payload),
                    ("hash", &format!("{:x}", md5::compute(&payload)))
                ])
                .send();

                match res {
                    Ok(r) => {
                        println!("submitted to serverdensity, status {:?}", r.status());
                    },
                    Err(e) => {
                        println!("failed to send to serverdensity, status {:?}", e.status());
                        println!("error: {:?}", e);
                    }
                }
            }

        }
    });

    let mut socket = UdpSocket::bind("127.0.0.1:1113").expect("could not listen on port 1113");

    loop {
        
        match read(&mut socket) {
            Err(_) => {
                println!("could not read from socket.\n");
                continue;
            },
            Ok(m) => {
                sender.send(m);
            }
        }
    }
}

fn read(socket : &mut UdpSocket) -> Result<Metric, String>
{
    let mut buf = [0; 300];
    let (amt, src) = try!(socket.recv_from(&mut buf).or_else(|_|Err("foo".to_string())));

    let count = BigEndian::read_i32(&buf[0..4]);
    let name = String::from_utf8_lossy(&buf[4..amt]).to_string();

    Ok(Metric {
        count: count,
        name
    })
}