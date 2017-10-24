<?php

use easybill\Metrics\ServerdensityUDPAgent\ServerdensityUDPAgent;

require __DIR__ . '/../../clients/php/src/ServerdensityUDPAgent.php';

$client = new ServerdensityUDPAgent();

while (true) {

    $i = 0;
    while ($i++ < 1000) {
        $client->sendSum('a', rand(10, 20)); // large number
        $client->sendAverage('b', rand(10, 20)); // probably ~15
        $client->sendPeak('c', rand(10, 20)); // probably ~20
        $client->sendMin('d', rand(10, 20));  // // probably ~10
    }
    echo ".";
    sleep(1);
}

