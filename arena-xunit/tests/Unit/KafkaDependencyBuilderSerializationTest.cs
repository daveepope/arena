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
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new KafkaDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("kafka", obj["type"]);
        Assert.Equal(9092, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new KafkaDependencyBuilder("test").WithPort(9192).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(9192, obj["port"]);
    }

    [Fact]
    public void Build_ApacheNativeFlavor_SerializesCorrectJson()
    {
        var dep = new KafkaDependencyBuilder("test").WithFlavor(KafkaFlavor.ApacheNative).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("apache_native", obj["flavor"]);
    }

    [Fact]
    public void Build_ConfluentFlavor_SerializesCorrectJson()
    {
        var dep = new KafkaDependencyBuilder("test").WithFlavor(KafkaFlavor.Confluent).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("confluent", obj["flavor"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new KafkaDependencyBuilder("test").Build();
        Assert.StartsWith("arena-kafka-", dep.Identifier);
    }
}
