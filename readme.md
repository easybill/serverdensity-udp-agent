# Basics

this project is not officially supported by server density.

## Arguments:

```
        --help
            Help :)

        --debug:
            Debug :)

        --account-url <account-url>
            Set this to your Server Density account url, e.g. example.serverdensity.io

        --agent-key <agent-key>
            This is the agent key used to identify the device when payloads are processed. You can find this in the
            top left corner when you view a device page in your UI

        --bind <bind>                                        Bind Address. [default: 127.0.0.1:1113]

        --serverdensity-endpoint <serverdensity-endpoint>
            Serverdensity API-Endpoint [default: https://api.serverdensity.io]

        --token <token>                                      Server Density API Token

         -c, --config <config>
            path to the serverdensity config file, may /etc/sd-agent/config.cfg?
```


## Installing

### linux64
Go to the releases tab and download the latest binary.

### OSX
clone the repository and run `cargo build --release`

## Running the Agent

The Udp-Server can read the serverdensity agent config file.
if the agent is already installed on your system you can simply run:

```
severdensity_udpserver
    --token=some_token \
    --config=./examples/serverdensity_config_file.cfg

```

you can also set the agent-key and the account-url.

```
severdensity_udpserver \
    --token=some_token \
    --agent-key=some_key \
    --account-url=[ACCOUNT].serverdensity.io
```

## Sending Metrics

The UDP-Server will aggregate these packages and send it to serverdensity.

From performance perspective you could send thousands of messages per second.


### php

we provide a small php client

```
composer require easybill/serverdensity_udp_metric_client
```

```php
<?php

$client = new ServerdensityUDPAgent();
$client->sendSum('[METRIC_GROUP].[METRIC]', 1);
```

### just udp

example php client:

```php

function send($metric, $count) {
    $host = '127.0.0.1';
    $port = '1113';

    $msg = pack('nN', 42, $count).$metric;

    $socket = socket_create(AF_INET, SOCK_DGRAM, SOL_UDP);
    socket_sendto($socket, $msg, strlen($msg), 0, $host, $port);
    socket_close($socket);
}

send('foo', 123);

```

# installing + Supervisor

```bash
wget https://github.com/easybill/serverdensity-udp-agent/releases/download/0.1/serverdensity_udpserver.zip
unzip serverdensity_udpserver.zip
rm serverdensity_udpserver.zip
mv serverdensity_udpserver /usr/local/bin/
```

now you can test if the server starts:

```
serverdensity_udpserver --token={SERVERDENSITY_TOKEN} --config=/etc/sd-agent/config.cfg
```


open `/etc/supervisor/conf.d/serverdensity_udpserver.conf` and add:

```
[program:serverdensity_udpserver]
command=serverdensity_udpserver --token={SERVERDENSITY_TOKEN} --config=/etc/sd-agent/config.cfg
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

check the update of the new process

`supervisorctl status | grep serverdensity_udpserver`
