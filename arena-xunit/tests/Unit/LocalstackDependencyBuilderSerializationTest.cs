using System;
using System.Linq;
using ArenaXunit.Dep;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class LocalstackDependencyBuilderSerializationTest
{
    [Fact]
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("localstack", obj["type"]);
        Assert.Equal(4566, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithPort(4577).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(4577, obj["port"]);
    }

    [Fact]
    public void Build_WithService_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithService("sqs").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["services"]);
        Assert.Equal("sqs", obj["services"][0]);
    }

    [Fact]
    public void Build_WithServices_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithServices("sqs", "s3").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["services"]);
        Assert.Equal(2, obj["services"].Count());
    }

    [Fact]
    public void Build_WithQueue_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithQueue("myqueue").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["queues"]);
        Assert.Equal("myqueue", obj["queues"][0]["name"]);
        Assert.Equal(false, obj["queues"][0]["fifo"]);
    }

    [Fact]
    public void Build_WithFifoQueue_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithFifoQueue("myqueue.fifo").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["queues"]);
        Assert.Equal("myqueue.fifo", obj["queues"][0]["name"]);
        Assert.Equal(true, obj["queues"][0]["fifo"]);
    }

    [Fact]
    public void Build_WithEventBus_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithEventBus("my-bus").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["event_buses"]);
        Assert.Equal("my-bus", obj["event_buses"][0]["name"]);
    }

    [Fact]
    public void Build_WithImage_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithImage("custom:tag").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("custom:tag", obj["image"]);
    }

    [Fact]
    public void Build_WithContainerName_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithContainerName("my-container").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-container", obj["container_name"]);
    }

    [Fact]
    public void Build_EndpointUrl_Correct()
    {
        var dep = new LocalstackDependencyBuilder("test").WithPort(4577).Build();
        Assert.Equal("http://localhost:4577", dep.EndpointUrl);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new LocalstackDependencyBuilder("test").Build();
        Assert.StartsWith("arena-localstack-", dep.Identifier);
    }

    [Fact]
    public void Build_WithEventRuleSqsTarget_SerializesCorrectJson()
    {
        var target = EventRuleTargetBuilder.SqsQueue("t1", "queue1");
        var rule = new EventRuleSpec
        {
            Name = "rule1",
            EventPattern = "{}",
            Targets = new System.Collections.Generic.List<EventRuleTarget> { target }
        };
        var dep = new LocalstackDependencyBuilder("test").WithEventRule(rule).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["event_rules"]);
        Assert.Equal("rule1", obj["event_rules"][0]["name"]);
        Assert.Equal("sqs_queue", obj["event_rules"][0]["targets"][0]["kind"]);
    }

    [Fact]
    public void Build_WithEventRuleLambdaTarget_SerializesCorrectJson()
    {
        var target = EventRuleTargetBuilder.Lambda("t2", "func1");
        var rule = new EventRuleSpec
        {
            Name = "rule2",
            EventBus = "my-bus",
            EventPattern = "{}",
            Targets = new System.Collections.Generic.List<EventRuleTarget> { target }
        };
        var dep = new LocalstackDependencyBuilder("test").WithEventRule(rule).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("lambda", obj["event_rules"][0]["targets"][0]["kind"]);
        Assert.Equal("func1", obj["event_rules"][0]["targets"][0]["function_name"]);
    }
}
