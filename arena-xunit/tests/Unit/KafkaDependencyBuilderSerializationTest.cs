using System;
using System.Linq;
using ArenaXunit.Dep;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class KafkaDependencyBuilderSerializationTest
{
    [Fact]
    public void build_default_port_serializes_correct_json()
    {
        var dep = new KafkaDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("kafka", obj["type"]);
        Assert.Equal(9092, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void build_custom_port_serializes_correct_json()
    {
        var dep = new KafkaDependencyBuilder("test").WithPort(9192).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(9192, obj["port"]);
    }

    [Fact]
    public void build_zookeeper_flavor_serializes_correct_json()
    {
        var dep = new KafkaDependencyBuilder("test").WithFlavor(KafkaFlavor.Zookeeper).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("zookeeper", obj["flavor"]);
    }

    [Fact]
    public void build_kraft_flavor_serializes_correct_json()
    {
        var dep = new KafkaDependencyBuilder("test").WithFlavor(KafkaFlavor.KRaft).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("kraft", obj["flavor"]);
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var dep = new KafkaDependencyBuilder("test").Build();
        Assert.StartsWith("arena-kafka-", dep.Identifier);
    }
}
