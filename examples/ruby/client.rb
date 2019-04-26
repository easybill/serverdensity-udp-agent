require 'socket'


class ServerdensityUDPAgent
  def send_raw(type, name, count)
    # the max length of a name is around 100 chars.
    (UDPSocket.new).send([type, count].pack('nN').concat(name), 0, "127.0.0.1", 1113)
  end

  # UDP Sender aggragates all events (1m) and flushs the SUM of all to serverdensity
  def send_sum(name, count)
    self.send_raw(42, name, count)
  end

  # UDP Sender aggragates all events (1m) and flushs the AGERAGE of all to serverdensity
  def send_average(name, count)
    self.send_raw(43, name, count)
  end

  # UDP Sender aggragates all events (1m) and flushs the PEAK of all to serverdensity
  # use this if you want to know the MAX value of something.
  def send_peak(name, count)
    self.send_raw(44, name, count)
  end

  # UDP Sender aggragates all events (1m) and flushs the MIN of all to serverdensity
  # use this if you want to know the MIN value of something.
  def send_min(name, count)
    self.send_raw(45, name, count)
  end
end

(ServerdensityUDPAgent.new).send_sum("foo.bar", 1)