# Prometheus UDP Monitor

## Installing

This project is build on each release for Linux & Mac x86, aarch64. You can download these pre-build binaries from
the [releases tab](https://github.com/easybill/openmetric-udp-agent/releases).

### Other Platforms

For other platforms you need to compile this lib yourself:

1. [Install Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
2. Clone this repository
3. Run `cargo b --release --bin=openmetrics_udpserver`
4. The executable is located in `target/release/openmetrics_udpserver`

## Sending Metrics

The UDP-Server will collect sent metrics and make them available through a http endpoint using the openmetrics-text
encoding. Sent values for the metric types Min, Average & Peak are just single values (if a value is received twice
before collection, the old value gets overridden). The Sum metric type will sum up all received values until a
collection happens - then the counter is reset to 0.

From performance perspective you could send thousands of messages per second.

### PHP

We provide a small php client

```
composer require easybill/serverdensity_udp_metric_client
```

```php
<?php

$client = new ServerdensityUDPAgent();
$client->sendSum('[METRIC_GROUP].[METRIC]', 1);
```

### Data Format

The data format that must be used to send data to the server must be as follows:

1. **u16**: representation of the metric type (see table below)
2. **i32**: the data count
3. the utf-8 encoded name of the metric

All numbers must be encoded using big endian byte order.

#### Metric Types:

| Type    | ID |
|---------|----|
| Sum     | 42 |
| Average | 43 |
| Peak    | 44 |
| Min     | 45 |

# Installing + Supervisor

```bash
# replace the download link for the required platform
wget https://github.com/easybill/openmetrics-udp-agent/releases/latest/download/openmetrics_udpserver_linux_x86_64
chmod +x sopenmetrics_udpserver
mv openmetrics_udpserver /usr/local/bin/
```

now you can test if the server starts:

```bash
./openmetrics_udpserver
```

open `/etc/supervisor/conf.d/openmetrics_udpserver.conf` and add:

```
[program:openmetrics_udpserver]
command=openmetrics_udpserver
user=sd-agent
process_name=%(program_name)s
numprocs=1
directory=/tmp
autostart=true
autorestart=true
startsecs=0
startretries=10
stdout_logfile=/var/log/supervisor/%(program_name)s.log
stderr_logfile=/var/log/supervisor/%(program_name)s_error.log
stopsignal=QUIT
```

Check the update of the new process

`supervisorctl status openmetrics_udpserver`

## Updating the udp server

```bash
wget https://github.com/easybill/openmetrics-udp-agent/releases/latest/download/openmetrics_udpserver_linux_x86_64
chmod +x openmetrics_udpserver
supervisorctl stop openmetrics_udpserver
rm /usr/local/bin/openmetrics_udpserver
mv openmetrics_udpserver /usr/local/bin/
supervisorctl start openmetrics_udpserver
supervisorctl status openmetrics_udpserver
```
