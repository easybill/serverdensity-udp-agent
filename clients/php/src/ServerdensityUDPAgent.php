<?php

namespace easybill\Metrics\ServerdensityUDPAgent;

final class ServerdensityUDPAgent
{
    private static $TYPE_SUM = 42;
    private static $TYPE_AVERAGE = 43;
    private static $TYPE_PEAK = 44;
    private static $TYPE_MIN = 45;

    /** @var int */
    private $port;

    public function __construct($port = 1113)
    {
        $this->port = $port;
    }

    private function send(int $type, string $name, int $count)
    {
        // just 127.0.0.1 is supported. udp ftw.
        $host = '127.0.0.1';

        $msg = pack('nN', $type, $count) . $name;

        $socket = socket_create(AF_INET, SOCK_DGRAM, SOL_UDP);
        socket_sendto($socket, $msg, strlen($msg), 0, $host, $this->port);
        socket_close($socket);
    }

    public function sendSum(string $name, int $count)
    {
        $this->send(self::$TYPE_SUM, $name, $count);
    }

    public function sendAverage(string $name, int $count)
    {
        $this->send(self::$TYPE_AVERAGE, $name, $count);
    }

    public function sendPeak(string $name, int $count)
    {
        $this->send(self::$TYPE_PEAK, $name, $count);
    }

    public function sendMin(string $name, int $count)
    {
        $this->send(self::$TYPE_MIN, $name, $count);
    }
}