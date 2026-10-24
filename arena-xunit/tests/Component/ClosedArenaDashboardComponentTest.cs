using System;
using System.Threading.Tasks;
using Xunit;

namespace ArenaDotnet.Xunit.ComponentTest;

public class ClosedArenaDashboardComponentTest
{
    private static ClosedArena PlainClosedArena(string name)
    {
        return new ClosedArena(name, new MatchBuilder(name + "-match").Build());
    }

    [Fact]
    public async Task WithLifecycleDashboard_OpenAndClose_ForwardsStateViaHttp()
    {
        using var server = new DashboardCaptureServer();

        var opened = await PlainClosedArena("xunit-visualize-forward").WithLifecycleDashboard(server.Url).OpenAsync();
        opened.Dispose();

        Assert.True(
            server.WaitForId("xunit-visualize-forward", TimeSpan.FromSeconds(2)),
            "no states were forwarded via http");
    }

    [Fact]
    public async Task WithLifecycleDashboard_DashboardUnavailable_OpensArenaAnyway()
    {
        var opened = await PlainClosedArena("xunit-visualize-unavailable")
            .WithLifecycleDashboard(1, null)
            .OpenAsync();
        opened.Dispose();
    }

    [Fact]
    public async Task WithLifecycleDashboard_AfterDispose_UnregistersForwarding()
    {
        using var server = new DashboardCaptureServer();

        var opened = await PlainClosedArena("xunit-visualize-teardown").WithLifecycleDashboard(server.Url).OpenAsync();
        opened.Dispose();
        Assert.True(server.WaitForId("xunit-visualize-teardown", TimeSpan.FromSeconds(2)));

        (await PlainClosedArena("xunit-visualize-teardown-unrelated").OpenAsync()).Dispose();
        await Task.Delay(200);

        Assert.DoesNotContain(
            "xunit-visualize-teardown-unrelated",
            server.Snapshot().ConvertAll(body => (string?)Newtonsoft.Json.Linq.JObject.Parse(body)["id"]));
    }
}
