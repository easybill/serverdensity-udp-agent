use std::thread;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::io::Read;
use regex::Regex;
use std::sync::mpsc::TryRecvError;
use config::Config;
use std::sync::mpsc::Receiver;
use ::Metric;
use ::md5;
use reqwest;
use udpserver::MetricType;
use handler::{SumHandler, AverageHandler, PeakHandler, MinHandler};

header! { (XForwardedHost, "X-Forwarded-Host") => [String] }

pub struct Aggregator<'a> {
    config: &'a Config,
    http_client: reqwest::Client,
    api_postback_uri: String 
}

impl<'a> Aggregator<'a> {
    
    pub fn new(config: &'a Config) -> Aggregator<'a> {
        Aggregator {
            config,
            http_client: reqwest::Client::new(),
            api_postback_uri: format!("{}/alerts/postbacks?token={}", &config.serverdensity_endpoint, &config.token)
        }
    }

    pub fn run(&self, receiver: Receiver<Metric>) {

        let regex = Regex::new(r"[^0-9a-zA-ZäöüÄÖÜß\-\(\)_]*").expect("failed to compile regex");

        let mut metricmap = HashMap::new();
        let mut sys_time = SystemTime::now();

        let handler_sum = SumHandler::new();
        let mut handler_avg = AverageHandler::new();
        let handler_peak = PeakHandler::new();
        let handler_min = MinHandler::new();

        loop {

            thread::sleep(Duration::from_millis(30));

            loop {

                let mut i = 0;

                match receiver.try_recv() {
                    Ok(metric) => {
                        let metric_name = regex.replace_all(&metric.name, "").trim().to_string();

                        if metric_name == "" {
                            println!("got empty metric name.");
                            continue;
                        }

                        if self.config.debug {
                            println!("Debug: got metric \"{}\" with count \"{}\"", metric_name, metric.count);
                        }

                        match metric.metric_type {
                            MetricType::SUM => {
                               handler_sum.handle(&metric_name, &metric, &mut metricmap);
                            },
                            MetricType::AVERAGE => {
                                handler_avg.handle(&metric_name, &metric, &mut metricmap);
                            },
                            MetricType::PEAK => {
                                handler_peak.handle(&metric_name, &metric, &mut metricmap);
                            },
                            MetricType::MIN => {
                                handler_min.handle(&metric_name, &metric, &mut metricmap);
                            }
                        };

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
                handler_sum.flush(&mut metricmap);
                handler_avg.flush(&mut metricmap);
                handler_peak.flush(&mut metricmap);
                handler_min.flush(&mut metricmap);
                self.push_to_serverdensity(&mut metricmap);
            }

        }
    }

    pub fn create_plugin_map(map : &HashMap<String, i32>) -> String
    {
        let mut outermap : HashMap<String, HashMap<String, i32>> = HashMap::new();

        for (k, v) in map {

            let len = k.len();

            match k.find(".") {
                Some(index) if index > 0 && index + 1 != len => {
                    outermap
                    .entry(k[..index].to_string())
                    .or_insert(HashMap::new())
                    .insert(k[index + 1..].to_string(), v.clone());
                },
                _ if k.trim() != "" => {
                    outermap
                    .entry("custom".to_string())
                    .or_insert(HashMap::new())
                    .insert(k.to_string(), v.clone());
                }
                _ => {}
            };
        }

        outermap.iter().map(|(k, v)|{
            let mut buf = String::new();
            buf.push_str("\"");
            buf.push_str(k);
            buf.push_str("\":{");

            buf.push_str(
                &v.iter().map(|(k, v)| {
                    format!("\"{}\":\"{}\"", k, v)
                })
                .collect::<Vec<String>>()
                .join(",")
                .to_string()
            );

            buf.push_str("}");
            buf
        })
        .collect::<Vec<String>>()
        .join(",")
        .to_string()
    }

    pub fn push_to_serverdensity(&self, metricmap : &mut HashMap<String, i32>)
    {
        if metricmap.len() == 0 {
            return;
        }

        
        let mut payload = "{\"agentKey\":\"".to_string();
        payload.push_str(&self.config.agent_key);
        payload.push_str("\",\"plugins\":{");
        payload.push_str(&Self::create_plugin_map(metricmap));
        payload.push_str("}}");

        *metricmap = HashMap::new();

        let send_data_to_backend_time = SystemTime::now();

        let data = &[
            ("payload", &payload),
            ("hash", &format!("{:x}", md5::compute(&payload)))
        ];

        if self.config.debug {
            println!("Data to send to Backend {:#?}", &data);
        }

        let mut res = self.http_client.post(&self.api_postback_uri)
        .header(XForwardedHost(self.config.account_url.clone()))
        .form(data)
        .send();

        let send_data_to_backend_tooked_in_ms = match send_data_to_backend_time.elapsed() {
            Ok(duration) => (duration.as_secs() * 1000) + (duration.subsec_nanos() as u64 / 1000000),
            Err(_) => {
                println!("seems to have trouble with the clock, should never happen.");
                return;
            }
        };
        
        match &mut res {
            &mut Ok(ref mut r) => {

                let mut content = String::new();
                match r.read_to_string(&mut content) {
                    Ok(_) => {
                        println!("submitted to serverdensity, tooked {}ms \n--- metrics --- \n{:#?} \n--- Request ---\n{:#?} \n\n{} \n----\n", &send_data_to_backend_tooked_in_ms, data, r, &content);
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
        };
            
    }

}

#[cfg(test)]
    mod tests {

        use ::std::collections::HashMap;
        use ::aggregator::Aggregator;

        #[test]
        fn it_works() {

            let mut m = HashMap::new();
            m.insert("foo".to_string(), 2);
            m.insert("foo.".to_string(), 3);
            m.insert(".foo.bar.barr".to_string(), 4);

            m.insert("foo.bar".to_string(), 5);
            m.insert("foo.bar.baz".to_string(), 6);

            let out = Aggregator::create_plugin_map(&m);

            println!("{}\n", &format!("{}", out));


            let mut m = HashMap::new();
            m.insert("foo".to_string(), 2);
            let out = Aggregator::create_plugin_map(&m);
            println!("{}\n", &format!("{}", out));

            let mut m = HashMap::new();
            m.insert("foo.bar".to_string(), 2);
            let out = Aggregator::create_plugin_map(&m);
            println!("{}\n", &format!("{}", out));

            let mut m = HashMap::new();
            let out = Aggregator::create_plugin_map(&m);
            println!("{}\n", &format!("{}", out));

            /*
            the assert doesnt work because the map is not ordered.

            assert_eq!(
                r#""foo":{"bar":"5","bar.baz":"6"},"custom":{".foo.bar.barr":"4","foo":"2","foo.":"3"}"#,
                &format!("{}", out)
            );
            */

        }
    }