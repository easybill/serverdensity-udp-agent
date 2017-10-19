extern crate byteorder;
extern crate clap;
extern crate url;
extern crate md5;
extern crate reqwest;
#[macro_use] extern crate hyper;
extern crate regex;
extern crate openssl_probe;

mod config;

use clap::{Arg, App};
use std::net::UdpSocket;
use byteorder::{BigEndian};
use byteorder::ByteOrder;
use std::sync::mpsc::channel;
use std::thread;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::io::Read;
use regex::Regex;
use std::sync::mpsc::TryRecvError;
use config::Config;

header! { (XForwardedHost, "X-Forwarded-Host") => [String] }


#[derive(Debug)]
struct Metric {
    pub name: String,
    pub count: i32
}

fn main() {

    ::openssl_probe::probe();

    let matches = App::new("Server Density UDP Monitor")
        .version("1.0")
        .author("Tim Glabisch. <serverdensity@tim.ainfach.de>")
        .about("UDP Sender for Serverdendity")
        .arg(Arg::with_name("token")
            .help("Server Density API Token")
            .long("token")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("account-url")
            .help("Set this to your Server Density account url, e.g. example.serverdensity.io")
            .long("account-url")
            .required(false)
            .takes_value(true))
        .arg(Arg::with_name("agent-key")
            .help("This is the agent key used to identify the device when payloads are processed. You can find this in the top left corner when you view a device page in your UI")
            .long("agent-key")
            .required(false)
            .takes_value(true))
        .arg(Arg::with_name("serverdensity-endpoint")
            .default_value("https://api.serverdensity.io")
            .help("Serverdensity API-Endpoint")
            .long("serverdensity-endpoint")
            .required(false)
            .takes_value(true))
        .arg(Arg::with_name("bind")
            .default_value("127.0.0.1:1113")
            .help("Bind Address.")
            .long("bind")
            .required(false)
            .takes_value(true))
        .arg(Arg::with_name("debug")
            .short("v")
            .help("verbose mode, just for debugging")
            .long("debug")
            .takes_value(false))
        .arg(Arg::with_name("config")
            .short("c")
            .help("path to the serverdensity config file, may /etc/sd-agent/config.cfg?")
            .long("config")
            .takes_value(true))
        .get_matches();

    let mut config = Config {
        token: matches.value_of("token").unwrap().to_string(),
        account_url: matches.value_of("account-url").unwrap_or("").to_string(),
        agent_key: matches.value_of("agent-key").unwrap_or("").to_string(),
        serverdensity_endpoint: matches.value_of("serverdensity-endpoint").unwrap().to_string(),
        debug: matches.is_present("debug"),
        bind: matches.value_of("bind").unwrap().to_string()
    };

    println!("UDP Sender for Serverdendity");
    
    if matches.is_present("config") {
        let config_file = matches.value_of("config").unwrap().to_string();
        match config.apply_config_file(&config_file) {
            Ok(_) => println!("successfully read config_file: {}", &config_file),
            Err(_) => {
                println!("could not read config_file: {}", &config_file);
                return;
            }
        };
    }

    println!("account-url: {}", &config.account_url);
    println!("agent-key: {}", &config.agent_key);
    println!("endpoint: {}", &config.serverdensity_endpoint);
    println!("debug: {:?}", &config.debug);
    println!("bind: {}", &config.bind);

    if config.agent_key.trim() == "" || config.account_url.trim() == "" {
        println!("agent-key or account-url not given.");
        return;
    }

    println!("\nServer is starting...");

    let (sender, receiver) = channel::<Metric>();

    let regex = Regex::new(r"[^0-9a-zA-ZäöüÄÖÜß\-\(\)_]*").expect("failed to compile regex");

    let thread_config = config.clone();
    thread::spawn(move|| {
        let config = thread_config;
        let mut metricmap = HashMap::new();
        let mut sys_time = SystemTime::now();
        let client = reqwest::Client::new();

        let uri = &format!("{}/alerts/postbacks?token={}", &config.serverdensity_endpoint, &config.token);

        loop {

            thread::sleep(Duration::from_millis(30));

            loop {

                let mut i = 0;

                match receiver.try_recv() {
                    Ok(metric) => {
                        let metric_name = regex.replace_all(&metric.name, "").to_string();

                        if metric_name.trim() == "" {
                            println!("got empty metric name.");
                            continue;
                        }

                        *metricmap.entry(metric_name).or_insert(0) += metric.count;

                        i = i + 1;

                        if i == 50_000 {
                            println!("got a lot of messages, may more than the server can handel...");
                        }
                    },
                    Err(TryRecvError::Empty) => {
                        // buffer ist leer.
                        break;
                    },
                    Err(TryRecvError::Disconnected) => {
                        panic!("channel disconnected, should never happen.");
                    }
                };

                if i != 0 && config.debug {
                    println!("got {} messages.", i);
                    println!("------------------\n{:#?}\n------------------", metricmap);
                }
            }

            let elapsed_time = match sys_time.elapsed() {
                Ok(t) => t,
                Err(_) => {
                    println!("seems to have trouble with the clock, should never happen.");
                    continue;
                }
            };
            
            if elapsed_time.as_secs() >= 10 {

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
                
                let mut payload = "{\"agentKey\":\"".to_string();
                payload.push_str(&config.agent_key);
                payload.push_str("\",\"plugins\":{\"website\":{");
                payload.push_str(&x);
                payload.push_str("}}}");


                metricmap = HashMap::new();

                let send_data_to_backend_time = SystemTime::now();

                let data = &[
                    ("payload", &payload),
                    ("hash", &format!("{:x}", md5::compute(&payload)))
                ];

                if config.debug {
                    println!("Data to send to Backend {:#?}", &data);
                }

                let mut res = client.post(uri)
                .header(XForwardedHost(config.account_url.clone()))
                .form(data)
                .send();

                let send_data_to_backend_tooked_in_ms = match send_data_to_backend_time.elapsed() {
                    Ok(duration) => (duration.as_secs() * 1000) + (duration.subsec_nanos() as u64 * 1000000),
                    Err(_) => {
                        println!("seems to have trouble with the clock, should never happen.");
                        continue;
                    }
                };
                
                if config.debug {
                    println!("println sending data to beackend tooked {}ms", &send_data_to_backend_tooked_in_ms);
                }

                match &mut res {
                    &mut Ok(ref mut r) => {

                        let mut content = String::new();
                        match r.read_to_string(&mut content) {
                            Ok(content) => {
                                println!("submitted to serverdensity, status {:?}, \n{:?}\n\n", r, content);
                            }, 
                            Err(_) => {
                                println!("submitted to serverdentity, status: {:?}, but could not read response.", r);
                            }
                        }
                    },
                    &mut Err(ref mut e) => {
                        println!("failed to send to serverdensity, status {:?}", e.status());
                        println!("error: {:?}", e);
                    }
                }
            }

        }
    });

    let mut socket = match UdpSocket::bind(&config.bind) {
        Ok(s) => s,
        Err(_) => {
            println!("could not listen, may someone is already listen on this port or the address is invalid?");
            return;
        }
    };

    loop {
        
        match read(&mut socket) {
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

fn read(socket : &mut UdpSocket) -> Result<Metric, String>
{
    let mut buf = [0; 300];
    let (amt, _) = try!(socket.recv_from(&mut buf).or_else(|_|Err("Could recv from Socket.".to_string())));

    if amt <= 6 {
        return Err("UDP Package size is to small.".to_string());
    }

    let metric_type = BigEndian::read_u16(&buf[0..2]);

    if metric_type != 42 {
        return Err("unsupported metric type".to_string());
    }

    let count = BigEndian::read_i32(&buf[2..6]);
    let name = String::from_utf8_lossy(&buf[6..amt]).to_string().replace("\"", "");

    Ok(Metric {
        count: count,
        name
    })
}