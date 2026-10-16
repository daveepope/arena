using System;
using System.Threading.Tasks;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ClosedArenaDashboardTest
{
    private static ClosedArena WithoutDashboard(string name) =>
        new(name, new MatchBuilder(name + "-match").Build());

    [Fact]
    public void WithLifecycleDashboard_NoArguments_ReturnsADifferentInstance()
    {
        var closed = WithoutDashboard("xunit-dashboard-default");

        Assert.NotSame(closed, closed.WithLifecycleDashboard());
    }

    [Fact]
    public void WithLifecycleDashboard_ExplicitUrl_ReturnsADifferentInstance()
    {
        var closed = WithoutDashboard("xunit-dashboard-url");

        Assert.NotSame(closed, closed.WithLifecycleDashboard("http://127.0.0.1:65535"));
    }

    [Fact]
    public void WithLifecycleDashboard_PortAndSearchDirectory_ReturnsADifferentInstance()
    {
        var closed = WithoutDashboard("xunit-dashboard-port");

        Assert.NotSame(closed, closed.WithLifecycleDashboard(1, "/nonexistent-search-dir"));
    }

    [Fact]
    public async Task OpenAsync_DashboardUnavailable_OpensTheArenaAnyway()
    {
        var closed = WithoutDashboard("xunit-dashboard-unavailable")
            .WithLifecycleDashboard(1, "/nonexistent-search-dir");

        var arena = await closed.OpenAsync();
        try
        {
            Assert.Equal("arena_open", arena.State().State);
        }
        finally
        {
            arena.Dispose();
        }
    }

    [Fact]
    public async Task OpenAsync_NoDashboardConfigured_OpensTheArenaAnyway()
    {
        var closed = WithoutDashboard("xunit-dashboard-absent");

        var arena = await closed.OpenAsync();
        try
        {
            Assert.Equal("arena_open", arena.State().State);
        }
        finally
        {
            arena.Dispose();
        }
    }
}
