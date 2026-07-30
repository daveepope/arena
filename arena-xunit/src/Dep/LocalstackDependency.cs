using System.Collections.Generic;
using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public class EventRuleTarget
{
    public string TargetId { get; set; } = default!;
    public string Kind { get; set; } = default!;
    public string QueueName { get; set; } = default!;
    public string FunctionName { get; set; } = default!;
}

public class EventRuleSpec
{
    public string Name { get; set; } = default!;
    public string? EventBus { get; set; }
    public string EventPattern { get; set; } = default!;
    public List<EventRuleTarget> Targets { get; set; } = default!;
}

public static class EventRuleTargetBuilder
{
    public static EventRuleTarget SqsQueue(string targetId, string queueName)
    {
        return new EventRuleTarget
        {
            TargetId = targetId,
            Kind = "sqs_queue",
            QueueName = queueName,
        };
    }

    public static EventRuleTarget Lambda(string targetId, string functionName)
    {
        return new EventRuleTarget
        {
            TargetId = targetId,
            Kind = "lambda",
            FunctionName = functionName,
        };
    }
}

public sealed class LocalstackDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "localstack";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;
    public string EndpointUrl => $"http://localhost:{Port}";

    internal LocalstackDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class LocalstackDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public LocalstackDependencyBuilder(string name)
    {
        _config["type"] = "localstack";
        _config["identifier"] = ArenaIdentifiers.Build("arena-localstack", name);
        _config["port"] = 4566;
    }

    public LocalstackDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }

    public LocalstackDependencyBuilder WithService(string service)
    {
        if (_config["services"] == null) _config["services"] = new JArray();
        ((JArray)_config["services"]!).Add(service);
        return this;
    }

    public LocalstackDependencyBuilder WithServices(params string[] services)
    {
        foreach (var s in services) WithService(s);
        return this;
    }

    public LocalstackDependencyBuilder WithQueue(string name)
    {
        if (_config["queues"] == null) _config["queues"] = new JArray();
        ((JArray)_config["queues"]!).Add(name);
        return this;
    }

    public LocalstackDependencyBuilder WithEventBus(string name)
    {
        if (_config["event_buses"] == null) _config["event_buses"] = new JArray();
        ((JArray)_config["event_buses"]!).Add(name);
        return this;
    }

    public LocalstackDependencyBuilder WithEventRule(EventRuleSpec spec)
    {
        if (_config["event_rules"] == null) _config["event_rules"] = new JArray();

        var targets = new JArray();
        foreach (var t in spec.Targets)
        {
            var target = new JObject { ["target_id"] = t.TargetId, ["kind"] = t.Kind };
            if (t.Kind == "sqs_queue") target["queue_name"] = t.QueueName;
            if (t.Kind == "lambda") target["function_name"] = t.FunctionName;
            targets.Add(target);
        }

        var rule = new JObject
        {
            ["name"] = spec.Name,
            ["event_pattern"] = spec.EventPattern,
            ["targets"] = targets
        };
        if (!string.IsNullOrEmpty(spec.EventBus)) rule["event_bus"] = spec.EventBus;

        ((JArray)_config["event_rules"]!).Add(rule);
        return this;
    }

    public LocalstackDependencyBuilder WithImage(string image) { _config["image"] = image; return this; }
    public LocalstackDependencyBuilder WithContainerName(string containerName) { _config["container_name"] = containerName; return this; }

    public LocalstackDependency Build() => new LocalstackDependency((JObject)_config.DeepClone());
}