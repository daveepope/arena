using System.Net;
using System.Net.Sockets;
using System.Collections.Generic;
using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaHostTest
{
    [Fact]
    public void FindAvailablePort_RandomStrategy_ReturnsPortWithinRange()
    {
        var port = ArenaHost.FindAvailablePort(24000, 24100, PortSearchStrategy.Random);
        Assert.InRange(port, 24000, 24099);
    }

    [Fact]
    public void FindAvailablePort_LinearStrategy_ReturnsPortWithinRange()
    {
        var port = ArenaHost.FindAvailablePort(24200, 24300, PortSearchStrategy.Linear);
        Assert.InRange(port, 24200, 24299);
    }

    [Fact]
    public void FindAvailablePort_ExhaustedRange_ThrowsArenaPortNotFoundException()
    {
        const int rangeStart = 24400;
        const int rangeEnd = 24402;
        var held = new List<TcpListener>();
        try
        {
            for (var p = rangeStart; p < rangeEnd; p++)
            {
                var listener = new TcpListener(IPAddress.Loopback, p);
                listener.Start();
                held.Add(listener);
            }

            Assert.Throws<ArenaPortNotFoundException>(
                () => ArenaHost.FindAvailablePort(rangeStart, rangeEnd, PortSearchStrategy.Linear));
        }
        finally
        {
            foreach (var listener in held)
            {
                listener.Stop();
            }
        }
    }

    [Fact]
    public void FindAvailablePort_InvertedRange_ThrowsArenaBindingErrorNotPortNotFound()
    {
        var ex = Assert.Throws<ArenaBindingError>(() => ArenaHost.FindAvailablePort(500, 500));
        Assert.IsNotType<ArenaPortNotFoundException>(ex);
    }
}
