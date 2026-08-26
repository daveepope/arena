using System;
using System.Linq;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

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
        var services = obj["services"];
        Assert.NotNull(services);
        Assert.Equal("sqs", services[0]);
    }

    [Fact]
    public void Build_WithServices_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithServices("sqs", "s3").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        var services = obj["services"];
        Assert.NotNull(services);
        Assert.Equal(2, services.Count());
    }

    [Fact]
    public void Build_WithQueue_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithQueue("myqueue").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        var queues = obj["queues"];
        Assert.NotNull(queues);
        var queue = Assert.Single(queues);
        Assert.Equal("myqueue", queue["name"]);
        Assert.Equal(false, queue["fifo"]);
    }

    [Fact]
    public void Build_WithFifoQueue_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithFifoQueue("myqueue.fifo").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        var queues = obj["queues"];
        Assert.NotNull(queues);
        var queue = Assert.Single(queues);
        Assert.Equal("myqueue.fifo", queue["name"]);
        Assert.Equal(true, queue["fifo"]);
    }

    [Fact]
    public void Build_WithEventBus_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithEventBus("my-bus").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        var eventBuses = obj["event_buses"];
        Assert.NotNull(eventBuses);
        var eventBus = Assert.Single(eventBuses);
        Assert.Equal("my-bus", eventBus["name"]);
    }

    [Fact]
    public void Build_WithImageName_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithImageName("localstack/localstack-pro").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("localstack/localstack-pro", obj["image_name"]);
    }

    [Fact]
    public void Build_WithImageTag_SerializesCorrectJson()
    {
        var dep = new LocalstackDependencyBuilder("test").WithImageTag("3.8").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("3.8", obj["image_tag"]);
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
        var eventRules = obj["event_rules"];
        Assert.NotNull(eventRules);
        var eventRule = Assert.Single(eventRules);
        Assert.Equal("rule1", eventRule["name"]);
        var targets = eventRule["targets"];
        Assert.NotNull(targets);
        var eventTarget = Assert.Single(targets);
        Assert.Equal("sqs_queue", eventTarget["kind"]);
    }

    [Fact]
    public void Build_WithLambda_SerializesCorrectJson()
    {
        var spec = new LambdaSpec(
            "my-fn",
            "python3.12",
            "handler.main",
            ".",
            new[] { new System.Collections.Generic.KeyValuePair<string, string>("KEY", "VALUE") });
        var dep = new LocalstackDependencyBuilder("test").WithLambda(spec).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        var lambdas = obj["lambdas"];
        Assert.NotNull(lambdas);
        var lambda = Assert.Single(lambdas);
        Assert.Equal("my-fn", lambda["name"]);
        Assert.Equal("python3.12", lambda["runtime"]);
        Assert.Equal("handler.main", lambda["handler"]);
        Assert.True(System.IO.Path.IsPathRooted((string)lambda["source_dir"]!));
        var environment = lambda["environment"];
        Assert.NotNull(environment);
        var pair = Assert.Single(environment);
        Assert.Equal("KEY", pair[0]);
        Assert.Equal("VALUE", pair[1]);
    }

    [Fact]
    public void Build_WithLambdaHomeRelativeSourceDir_ExpandsToUserProfile()
    {
        var spec = new LambdaSpec("my-fn", "python3.12", "handler.main", "~/my-lambda-src");
        var dep = new LocalstackDependencyBuilder("test").WithLambda(spec).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        var lambda = Assert.Single(obj["lambdas"]!);
        var sourceDir = (string)lambda["source_dir"]!;
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        Assert.StartsWith(home, sourceDir);
        Assert.DoesNotContain("~", sourceDir);
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
        var eventRules = obj["event_rules"];
        Assert.NotNull(eventRules);
        var eventRule = Assert.Single(eventRules);
        var targets = eventRule["targets"];
        Assert.NotNull(targets);
        var eventTarget = Assert.Single(targets);
        Assert.Equal("lambda", eventTarget["kind"]);
        Assert.Equal("func1", eventTarget["function_name"]);
    }
}
