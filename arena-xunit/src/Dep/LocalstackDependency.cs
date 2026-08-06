using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Dep;

public sealed class QueueSpec
{
    public string Name { get; }
    public bool Fifo { get; }

    public QueueSpec(string name, bool fifo = false)
    {
        Name = name;
        Fifo = fifo;
    }
}

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
    public List<EventRuleTarget> Targets { get; set; } = new();
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

[JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
internal sealed class LocalstackRuleConfig
{
    [JsonProperty("name")] public string Name { get; set; } = default!;
    [JsonProperty("event_bus")] public string? EventBus { get; set; }
    [JsonProperty("event_pattern")] public string EventPattern { get; set; } = default!;
    [JsonProperty("targets")] public List<LocalstackTargetConfig>? Targets { get; set; }
}

[JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
internal sealed class LocalstackTargetConfig
{
    [JsonProperty("target_id")] public string TargetId { get; set; } = default!;
    [JsonProperty("kind")] public string Kind { get; set; } = default!;
    [JsonProperty("queue_name")] public string? QueueName { get; set; }
    [JsonProperty("function_name")] public string? FunctionName { get; set; }
}

[JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
internal sealed class LocalstackQueueConfig
{
    [JsonProperty("name")] public string Name { get; set; } = default!;
    [JsonProperty("fifo")] public bool Fifo { get; set; }
}

[JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
internal sealed class LocalstackEventBusConfig
{
    [JsonProperty("name")] public string Name { get; set; } = default!;
}

public sealed class LocalstackDependency : IArenaMatchPiece
{
    public string Type => "localstack";
    public string Identifier { get; }
    public int Port { get; }
    public string EndpointUrl { get; }

    private readonly List<string> _services;
    private readonly List<LocalstackQueueConfig> _queues;
    private readonly List<LocalstackEventBusConfig> _eventBuses;
    private readonly List<LocalstackRuleConfig> _eventRules;
    private readonly string? _image;
    private readonly string? _containerName;

    internal LocalstackDependency(
        string identifier,
        int port,
        List<string> services,
        List<LocalstackQueueConfig> queues,
        List<LocalstackEventBusConfig> eventBuses,
        List<LocalstackRuleConfig> eventRules,
        string? image,
        string? containerName)
    {
        Identifier = identifier;
        Port = port;
        EndpointUrl = $"http://localhost:{port}";
        _services = services;
        _queues = queues;
        _eventBuses = eventBuses;
        _eventRules = eventRules;
        _image = image;
        _containerName = containerName;
    }

    public string ForFfi()
    {
        var config = new LocalstackConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
        };

        if (_services.Count > 0) config.Services = _services;
        if (_queues.Count > 0) config.Queues = _queues;
        if (_eventBuses.Count > 0) config.EventBuses = _eventBuses;
        if (_eventRules.Count > 0) config.EventRules = _eventRules;
        if (!string.IsNullOrEmpty(_image)) config.Image = _image;
        if (!string.IsNullOrEmpty(_containerName)) config.ContainerName = _containerName;

        return ArenaJson.Serialize(config);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class LocalstackConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
        [JsonProperty("services")] public List<string>? Services { get; set; }
        [JsonProperty("queues")] public List<LocalstackQueueConfig>? Queues { get; set; }
        [JsonProperty("event_buses")] public List<LocalstackEventBusConfig>? EventBuses { get; set; }
        [JsonProperty("event_rules")] public List<LocalstackRuleConfig>? EventRules { get; set; }
        [JsonProperty("image")] public string? Image { get; set; }
        [JsonProperty("container_name")] public string? ContainerName { get; set; }
    }
}

public sealed class LocalstackDependencyBuilder
{
    private readonly string _name;
    private int _port = 4566;
    private readonly List<string> _services = new();
    private readonly List<QueueSpec> _queues = new();
    private readonly List<string> _eventBuses = new();
    private readonly List<EventRuleSpec> _eventRules = new();
    private string? _image;
    private string? _containerName;

    public LocalstackDependencyBuilder(string name)
    {
        _name = name;
    }

    public LocalstackDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public LocalstackDependencyBuilder WithService(string service)
    {
        _services.Add(service);
        return this;
    }

    public LocalstackDependencyBuilder WithServices(params string[] services)
    {
        _services.AddRange(services);
        return this;
    }

    public LocalstackDependencyBuilder WithQueue(string name)
    {
        return WithQueueSpec(new QueueSpec(name));
    }

    public LocalstackDependencyBuilder WithFifoQueue(string name)
    {
        return WithQueueSpec(new QueueSpec(name, fifo: true));
    }

    public LocalstackDependencyBuilder WithQueueSpec(QueueSpec spec)
    {
        _queues.Add(spec);
        return this;
    }

    public LocalstackDependencyBuilder WithEventBus(string name)
    {
        _eventBuses.Add(name);
        return this;
    }

    public LocalstackDependencyBuilder WithEventRule(EventRuleSpec spec)
    {
        _eventRules.Add(spec);
        return this;
    }

    public LocalstackDependencyBuilder WithImage(string image)
    {
        _image = image;
        return this;
    }

    public LocalstackDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public LocalstackDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-localstack", _name);
        var rules = new List<LocalstackRuleConfig>();
        foreach (var spec in _eventRules)
        {
            var targets = new List<LocalstackTargetConfig>();
            foreach (var t in spec.Targets)
            {
                targets.Add(new LocalstackTargetConfig
                {
                    TargetId = t.TargetId,
                    Kind = t.Kind,
                    QueueName = t.Kind == "sqs_queue" ? t.QueueName : null,
                    FunctionName = t.Kind == "lambda" ? t.FunctionName : null,
                });
            }
            rules.Add(new LocalstackRuleConfig
            {
                Name = spec.Name,
                EventBus = spec.EventBus,
                EventPattern = spec.EventPattern,
                Targets = targets,
            });
        }

        var queues = new List<LocalstackQueueConfig>();
        foreach (var spec in _queues)
        {
            queues.Add(new LocalstackQueueConfig { Name = spec.Name, Fifo = spec.Fifo });
        }

        var eventBuses = new List<LocalstackEventBusConfig>();
        foreach (var name in _eventBuses)
        {
            eventBuses.Add(new LocalstackEventBusConfig { Name = name });
        }

        return new LocalstackDependency(
            identifier,
            _port,
            new List<string>(_services),
            queues,
            eventBuses,
            rules,
            _image,
            _containerName);
    }
}
