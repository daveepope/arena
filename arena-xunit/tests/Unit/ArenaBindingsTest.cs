using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaBindingsTest
{
    [Theory]
    [InlineData(-1)]
    [InlineData(65536)]
    public void DashboardEnsureRunning_PortOutOfRange_ThrowsArenaBindingError(int port)
    {
        var error = Assert.Throws<ArenaBindingError>(
            () => ArenaBindings.DashboardEnsureRunning(port, null));

        Assert.Contains("dashboard port out of range", error.Message);
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    public void RegisterDashboardObserver_BlankUrl_ThrowsArenaBindingError(string? url)
    {
        var error = Assert.Throws<ArenaBindingError>(
            () => ArenaBindings.RegisterDashboardObserver(url!));

        Assert.Contains("dashboard url is null or empty", error.Message);
    }

    [Fact]
    public void RemoveLifecycleObserver_ZeroToken_ReturnsWithoutThrowing()
    {
        Assert.Null(Record.Exception(() => ArenaBindings.RemoveLifecycleObserver(0)));
    }
}
