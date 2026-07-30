using ArenaXunit.Dep;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class TemporalDependencyBuilderSerializationTest
{
    [Fact]
    public void build_default_port_serializes_correct_json()
    {
        var dep = new TemporalDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("temporal", obj["type"]);
        Assert.Equal(7233, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void build_custom_port_serializes_correct_json()
    {
        var dep = new TemporalDependencyBuilder("test").WithPort(7333).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(7333, obj["port"]);
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var dep = new TemporalDependencyBuilder("test").Build();
        Assert.StartsWith("arena-temporal-", dep.Identifier);
    }
}
