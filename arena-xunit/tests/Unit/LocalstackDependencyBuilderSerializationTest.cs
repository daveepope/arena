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
    public void build_default_port_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("localstack", obj["type"]);
        Assert.Equal(4566, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void build_custom_port_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").WithPort(4577).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(4577, obj["port"]);
    }

    [Fact]
    public void build_with_service_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").WithService("sqs").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["services"]);
        Assert.Equal("sqs", obj["services"][0]);
    }

    [Fact]
    public void build_with_services_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").WithServices("sqs", "s3").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["services"]);
        Assert.Equal(2, obj["services"].Count());
    }

    [Fact]
    public void build_with_queue_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").WithQueue("myqueue").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["queues"]);
        Assert.Equal("myqueue", obj["queues"][0]);
    }

    [Fact]
    public void build_with_event_bus_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").WithEventBus("my-bus").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["event_buses"]);
        Assert.Equal("my-bus", obj["event_buses"][0]);
    }

    [Fact]
    public void build_with_image_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").WithImage("custom:tag").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("custom:tag", obj["image"]);
    }

    [Fact]
    public void build_with_container_name_serializes_correct_json()
    {
        var dep = new LocalstackDependencyBuilder("test").WithContainerName("my-container").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-container", obj["container_name"]);
    }

    [Fact]
    public void build_endpoint_url_correct()
    {
        var dep = new LocalstackDependencyBuilder("test").WithPort(4577).Build();
        Assert.Equal("http://localhost:4577", dep.EndpointUrl);
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var dep = new LocalstackDependencyBuilder("test").Build();
        Assert.StartsWith("arena-localstack-", dep.Identifier);
    }

    [Fact]
    public void build_with_event_rule_sqs_target_serializes_correct_json()
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
    public void build_with_event_rule_lambda_target_serializes_correct_json()
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
