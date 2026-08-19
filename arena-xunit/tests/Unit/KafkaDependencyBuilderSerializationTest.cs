using System;
using System.Linq;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

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

    [Fact]
    public void Build_WithImageName_SerializesCorrectJson()
    {
        var dep = new KafkaDependencyBuilder("test").WithImageName("apache/kafka").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("apache/kafka", obj["image_name"]);
    }

    [Fact]
    public void Build_WithContainerName_SerializesCorrectJson()
    {
        var dep = new KafkaDependencyBuilder("test").WithContainerName("my-kafka").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-kafka", obj["container_name"]);
    }

    [Fact]
    public void Build_WithTopic_SerializesCorrectJson()
    {
        var dep = new KafkaDependencyBuilder("test").WithTopic("orders").WithTopic("payments").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        var topics = obj["topics"];
        Assert.NotNull(topics);
        Assert.Equal(2, topics.Count());
        Assert.Equal("orders", topics[0]);
        Assert.Equal("payments", topics[1]);
    }

    [Fact]
    public void Build_NoTopics_OmitsTopicsFromJson()
    {
        var dep = new KafkaDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Null(obj["topics"]);
    }

    [Fact]
    public void Build_ThenMutateBuilderTopics_BuiltDependencyUnaffected()
    {
        var builder = new KafkaDependencyBuilder("test").WithTopic("orders");
        var dep = builder.Build();
        builder.WithTopic("payments");
        Assert.Single(dep.Topics!);
        Assert.Equal("orders", dep.Topics![0]);
    }
}
