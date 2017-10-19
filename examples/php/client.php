<?php


function send($metric, $count) {
    $host = '127.0.0.1';
    $port = '1113';

    $msg = pack('nN', 42, $count).$metric;

    $socket = socket_create(AF_INET, SOCK_DGRAM, SOL_UDP);
    socket_sendto($socket, $msg, strlen($msg), 0, $host, $port);
    socket_close($socket);
}

while(true) {

    $i = 0;
    while($i++ < 1000) {
        send('a', 1);
        send('b', 1);
        send('c', 1);
    }
    echo ".";
    sleep(1);
}

