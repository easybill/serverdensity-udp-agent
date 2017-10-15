<?php

if (!($socket = stream_socket_server("udp://127.0.0.1:1113", $errno, $errstr, STREAM_SERVER_BIND))) {
    die("cant create socket, $errstr ($errno)");
}


$buffer = [];

do {
    $pkt = stream_socket_recvfrom($socket, 512, 0, $peer);

    $unpack = unpack('N', substr($pkt, 0, 4));

    if (!isset($unpack[1])) {
        continue;
    }

    $count = $unpack[1];
    $metric = substr($pkt, 4);

    if (!isset($buffer[$metric])) {
        $buffer[$metric] = [];
    }

    $buffer[$metric][] = $count;

    echo "$peer: count $count, metric: $metric";
    echo "\n\n";

} while ($pkt !== false);
