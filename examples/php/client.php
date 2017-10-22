<?php

const TYPE_SUM = 42;
const TYPE_AVERAGE = 43;
const TYPE_PEAK = 44;
const TYPE_MIN = 45;

function send($type, $metric, $count) {
    $host = '127.0.0.1';
    $port = '1113';

    $msg = pack('nN', $type, $count).$metric;

    $socket = socket_create(AF_INET, SOCK_DGRAM, SOL_UDP);
    socket_sendto($socket, $msg, strlen($msg), 0, $host, $port);
    socket_close($socket);
}

while(true) {

    $i = 0;
    while($i++ < 1000) {
        send(TYPE_SUM, 'a', 1);
        send(TYPE_AVERAGE, 'b', rand(10, 20));
        send(TYPE_PEAK, 'c', rand(10, 20));
        send(TYPE_MIN, 'd', rand(10, 20));
    }
    echo ".";
    sleep(1);
}

