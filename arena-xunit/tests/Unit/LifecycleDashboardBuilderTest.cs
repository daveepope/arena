using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class LifecycleDashboardBuilderTest
{
    private static ClosedArena Unprofiled() =>
        new("xunit-dashboard-builder", new MatchBuilder("xunit-dashboard-builder-match").Build());

    [Fact]
    public void Done_AfterWithCpuProfiler_ReturnsAnArenaCarryingTheProfiler()
    {
        var configured = Unprofiled()
            .WithLifecycleDashboard("http://127.0.0.1:65535")
            .WithCpuProfiler("svc")
            .Done();

        Assert.NotNull(JObject.Parse(configured.MatchConfigForFfi())["cpu_profiled_components"]);
    }

    [Fact]
    public void WithCpuProfiler_Called_LeavesTheOriginatingArenaUnprofiled()
    {
        var original = Unprofiled();

        var configured = original.WithLifecycleDashboard("http://127.0.0.1:65535")
            .WithCpuProfiler("svc")
            .Done();

        Assert.NotSame(original, configured);
        Assert.Null(JObject.Parse(original.MatchConfigForFfi())["cpu_profiled_components"]);
    }
}
