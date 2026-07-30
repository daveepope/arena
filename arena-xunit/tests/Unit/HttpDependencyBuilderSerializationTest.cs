using System;
using System.Linq;
using ArenaXunit.Dep;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class HttpDependencyBuilderSerializationTest
{
    [Fact]
    public void build_default_port_serializes_correct_json()
    {
        var dep = new HttpDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("http", obj["type"]);
        Assert.Equal(8080, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void build_custom_port_serializes_correct_json()
    {
        var dep = new HttpDependencyBuilder("test").WithPort(9900).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(9900, obj["port"]);
    }

    [Fact]
    public void build_with_listen_ip_serializes_correct_json()
    {
        var dep = new HttpDependencyBuilder("test").WithListenIp("0.0.0.0").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("0.0.0.0", obj["listen_ip"]);
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var dep = new HttpDependencyBuilder("test").Build();
        Assert.StartsWith("arena-http-", dep.Identifier);
    }
}
