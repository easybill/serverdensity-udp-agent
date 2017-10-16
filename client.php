<?php


function send($metric, $count) {
    
        $host = '127.0.0.1';
        $port = '1113';
    
        $msg = pack('N', $count).$metric;
    
        $socket = socket_create(AF_INET, SOCK_DGRAM, SOL_UDP);
        socket_sendto($socket, $msg, strlen($msg), 0, $host, $port);
        socket_close($socket);
    }

while(true) {

    send('foo', mt_rand(1, 500));
    usleep(mt_rand(30000, 100000));
    echo ".";
}

