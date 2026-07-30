using System;
using System.Net;
using System.Net.Sockets;

namespace ArenaExamples.Test.Shared;

public static class EphemeralTestRuntime
{
    private static readonly object Lock = new object();
    private static int? _nextPort;
    private static readonly string Suffix = Guid.NewGuid().ToString("N")[..8];

    public static int AllocatePort()
    {
        lock (Lock)
        {
            if (_nextPort.HasValue)
            {
                var p = _nextPort.Value;
                _nextPort = p + 1;
                return p;
            }
            var port = FindOpenPort();
            _nextPort = port + 1;
            return port;
        }
    }

    public static string NetworkName => $"arena-example-api-network-{Suffix}";

    private static int FindOpenPort()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        listener.Stop();
        return port;
    }
}
