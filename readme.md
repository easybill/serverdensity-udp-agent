# Basics

this project is not official supported by serverdensity.

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

go to the releases tab and download the latest binary.

## Running the Agent

severdensity_udpserver can read the serverdensity configurationfile.
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

this will send a foo metric to serverdensity_udpserver.
serverdensity_udpserver will aggregate these packages and send it to serverdensity.

from performance perspective you could send thousands of messages per second.
