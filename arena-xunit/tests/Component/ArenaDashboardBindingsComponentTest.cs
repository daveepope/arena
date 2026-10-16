using System;
using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.ComponentTest;

public class ArenaDashboardBindingsComponentTest
{
    [Fact]
    public void RegisterDashboardObserver_ValidUrl_ReturnsRemovableToken()
    {
        var token = ArenaBindings.RegisterDashboardObserver("http://127.0.0.1:1");

        Assert.NotEqual(0UL, token);
        ArenaBindings.RemoveLifecycleObserver(token);
    }

    [Fact]
    public void RegisterDashboardObserver_EmptyUrl_ThrowsArenaBindingError()
    {
        Assert.Throws<ArenaBindingError>(() => ArenaBindings.RegisterDashboardObserver(""));
    }

    [Fact]
    public void DashboardEnsureRunning_NoServerAndNoSearchDir_ThrowsArenaBindingError()
    {
        Assert.Throws<ArenaBindingError>(() => ArenaBindings.DashboardEnsureRunning(1, null));
    }

    [Fact]
    public void DashboardEnsureRunning_NegativePort_ThrowsArenaBindingError()
    {
        Assert.Throws<ArenaBindingError>(() => ArenaBindings.DashboardEnsureRunning(-1, null));
    }

    [Fact]
    public void DashboardEnsureRunning_PortAboveRange_ThrowsArenaBindingError()
    {
        Assert.Throws<ArenaBindingError>(() => ArenaBindings.DashboardEnsureRunning(65536, null));
    }

    [Fact]
    public void RegisterDashboardObserver_OpenAndClose_ForwardsStateViaHttp()
    {
        using var server = new DashboardCaptureServer();

        var token = ArenaBindings.RegisterDashboardObserver(server.Url);
        try
        {
            var handle = ArenaBindings.OpenArena("xunit-dashboard-forward", "{}", ArenaLogLevel.Info);
            ArenaBindings.CloseArena(handle);
        }
        finally
        {
            ArenaBindings.RemoveLifecycleObserver(token);
        }

        Assert.True(
            server.WaitForId("xunit-dashboard-forward", TimeSpan.FromSeconds(2)),
            "no states were forwarded via http");
    }
}
