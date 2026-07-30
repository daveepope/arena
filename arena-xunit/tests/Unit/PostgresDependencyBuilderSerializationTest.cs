using System;
using System.Linq;
using ArenaXunit.Dep;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class PostgresDependencyBuilderSerializationTest
{
    [Fact]
    public void build_default_port_serializes_correct_json()
    {
        var dep = new PostgresDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("postgres", obj["type"]);
        Assert.Equal(5432, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void build_custom_port_serializes_correct_json()
    {
        var dep = new PostgresDependencyBuilder("test").WithPort(5532).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(5532, obj["port"]);
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var dep = new PostgresDependencyBuilder("test").Build();
        Assert.StartsWith("arena-postgres-", dep.Identifier);
    }
}
