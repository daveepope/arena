using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ClosedArenaCpuProfilerTest
{
    private static ClosedArena Unprofiled(string name) =>
        new(name, new MatchBuilder(name + "-match").Build());

    private static LifecycleDashboardBuilder WithDashboard(string name) =>
        Unprofiled(name).WithLifecycleDashboard("http://127.0.0.1:65535");

    [Fact]
    public void MatchConfigForFfi_NoCpuProfiler_OmitsTheCpuProfiledComponentsField()
    {
        var closed = Unprofiled("xunit-profiler-none");

        Assert.Null(JObject.Parse(closed.MatchConfigForFfi())["cpu_profiled_components"]);
    }

    [Fact]
    public void WithCpuProfiler_SingleComponent_SerializesThatId()
    {
        var closed = WithDashboard("xunit-profiler-one").WithCpuProfiler("svc").Done();

        var ids = JObject.Parse(closed.MatchConfigForFfi())["cpu_profiled_components"];
        Assert.Equal(new[] { "svc" }, ids!.ToObject<string[]>());
    }

    [Fact]
    public void WithCpuProfiler_SeveralComponents_PreserveDeclarationOrder()
    {
        var closed = WithDashboard("xunit-profiler-many")
            .WithCpuProfiler("svc")
            .WithCpuProfiler("worker")
            .Done();

        var ids = JObject.Parse(closed.MatchConfigForFfi())["cpu_profiled_components"];
        Assert.Equal(new[] { "svc", "worker" }, ids!.ToObject<string[]>());
    }

    [Fact]
    public void WithCpuProfileLimitBytes_Set_SerializesTheLimit()
    {
        var closed = WithDashboard("xunit-profiler-limit")
            .WithCpuProfiler("svc")
            .WithCpuProfileLimitBytes(1024)
            .Done();

        Assert.Equal(
            1024L,
            JObject.Parse(closed.MatchConfigForFfi())["cpu_profile_limit_bytes"]!.Value<long>());
    }

    [Fact]
    public void MatchConfigForFfi_NoCpuProfileLimit_OmitsTheCpuProfileLimitField()
    {
        var closed = WithDashboard("xunit-profiler-nolimit").WithCpuProfiler("svc").Done();

        Assert.Null(JObject.Parse(closed.MatchConfigForFfi())["cpu_profile_limit_bytes"]);
    }
}
