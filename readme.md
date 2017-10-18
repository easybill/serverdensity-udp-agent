# Basics


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
```

## Example

severdensity_udpserver \
            --token=some_token \
            --agent-key=some_key \
            --account-url=[ACCOUNT].serverdensity.io


## Installing

