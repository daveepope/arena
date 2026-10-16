using System;
using System.Collections.Generic;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.ComponentTest;

internal sealed class DashboardCaptureServer : IDisposable
{
    private readonly TcpListener _listener;
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _acceptLoop;
    private readonly List<string> _bodies = new();

    public DashboardCaptureServer()
    {
        _listener = new TcpListener(IPAddress.Loopback, 0);
        _listener.Start();
        Url = $"http://127.0.0.1:{((IPEndPoint)_listener.LocalEndpoint).Port}";
        _acceptLoop = Task.Run(() => AcceptLoopAsync(_cts.Token));
    }

    public string Url { get; }

    public List<string> Snapshot()
    {
        lock (_bodies)
        {
            return new List<string>(_bodies);
        }
    }

    public bool WaitForId(string id, TimeSpan timeout)
    {
        var deadline = DateTime.UtcNow + timeout;
        do
        {
            foreach (var body in Snapshot())
            {
                if ((string?)JObject.Parse(body)["id"] == id)
                    return true;
            }
            Thread.Sleep(10);
        } while (DateTime.UtcNow < deadline);
        return false;
    }

    private async Task AcceptLoopAsync(CancellationToken token)
    {
        while (!token.IsCancellationRequested)
        {
            TcpClient client;
            try
            {
                client = await _listener.AcceptTcpClientAsync();
            }
            catch (Exception)
            {
                return;
            }
            _ = HandleClientAsync(client);
        }
    }

    private async Task HandleClientAsync(TcpClient client)
    {
        try
        {
            using (client)
            using (var stream = client.GetStream())
            {
                var buffer = new List<byte>();
                var chunk = new byte[4096];
                int headerEnd;
                while ((headerEnd = IndexOfDoubleCrlf(buffer)) < 0)
                {
                    int read = await stream.ReadAsync(chunk, 0, chunk.Length);
                    if (read == 0)
                        return;
                    buffer.AddRange(new ArraySegment<byte>(chunk, 0, read));
                }
                var headerText = Encoding.ASCII.GetString(buffer.ToArray(), 0, headerEnd);
                int contentLength = ParseContentLength(headerText);
                int bodyStart = headerEnd + 4;
                while (buffer.Count - bodyStart < contentLength)
                {
                    int read = await stream.ReadAsync(chunk, 0, chunk.Length);
                    if (read == 0)
                        break;
                    buffer.AddRange(new ArraySegment<byte>(chunk, 0, read));
                }
                int available = Math.Min(contentLength, buffer.Count - bodyStart);
                var body = Encoding.UTF8.GetString(buffer.ToArray(), bodyStart, available);
                lock (_bodies)
                {
                    _bodies.Add(body);
                }
                var response = Encoding.ASCII.GetBytes(
                    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                await stream.WriteAsync(response, 0, response.Length);
            }
        }
        catch (Exception)
        {
        }
    }

    private static int IndexOfDoubleCrlf(List<byte> buffer)
    {
        for (int i = 0; i + 3 < buffer.Count; i++)
        {
            if (buffer[i] == (byte)'\r' && buffer[i + 1] == (byte)'\n'
                && buffer[i + 2] == (byte)'\r' && buffer[i + 3] == (byte)'\n')
                return i;
        }
        return -1;
    }

    private static int ParseContentLength(string headerText)
    {
        foreach (var line in headerText.Split("\r\n"))
        {
            if (line.StartsWith("Content-Length:", StringComparison.OrdinalIgnoreCase))
                return int.Parse(line.Substring("Content-Length:".Length).Trim());
        }
        return 0;
    }

    public void Dispose()
    {
        _cts.Cancel();
        _listener.Stop();
    }
}
