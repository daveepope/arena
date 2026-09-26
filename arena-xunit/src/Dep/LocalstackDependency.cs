using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

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

public sealed class LambdaSpec
{
    public string Name { get; }
    public string Runtime { get; }
    public string Handler { get; }
    public string SourceDir { get; }
    public IReadOnlyList<KeyValuePair<string, string>> Environment { get; }

    public LambdaSpec(
        string name,
        string runtime,
        string handler,
        string sourceDir,
        IEnumerable<KeyValuePair<string, string>>? environment = null)
    {
        Name = name;
        Runtime = runtime;
        Handler = handler;
        SourceDir = sourceDir;
        Environment = (environment ?? Enumerable.Empty<KeyValuePair<string, string>>()).ToList();
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
internal sealed class LocalstackLambdaConfig
{
    [JsonProperty("name")] public string Name { get; set; } = default!;
    [JsonProperty("runtime")] public string Runtime { get; set; } = default!;
    [JsonProperty("handler")] public string Handler { get; set; } = default!;
    [JsonProperty("source_dir")] public string SourceDir { get; set; } = default!;
    [JsonProperty("environment")] public List<List<string>> Environment { get; set; } = new();
}

[JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
internal sealed class LocalstackEventBusConfig
{
    [JsonProperty("name")] public string Name { get; set; } = default!;
}

public sealed class LocalstackDependency : IArenaDependency
{
    public string Type => "localstack";
    public string Identifier { get; }
    public long? ExpirySeconds { get; internal set; }
    public int Port { get; }
    public string EndpointUrl { get; }

    private readonly List<string> _services;
    private readonly List<LocalstackQueueConfig> _queues;
    private readonly List<LambdaSpec> _lambdas;
    private readonly List<LocalstackEventBusConfig> _eventBuses;
    private readonly List<LocalstackRuleConfig> _eventRules;
    private readonly string? _imageName;
    private readonly string? _imageTag;
    private readonly string? _containerName;
    private readonly List<IArenaDependency> _children;

    internal LocalstackDependency(
        string identifier,
        int port,
        List<string> services,
        List<LocalstackQueueConfig> queues,
        List<LambdaSpec> lambdas,
        List<LocalstackEventBusConfig> eventBuses,
        List<LocalstackRuleConfig> eventRules,
        string? imageName,
        string? imageTag,
        string? containerName,
        List<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        EndpointUrl = $"http://localhost:{port}";
        _services = services;
        _queues = queues;
        _lambdas = lambdas;
        _eventBuses = eventBuses;
        _eventRules = eventRules;
        _imageName = imageName;
        _imageTag = imageTag;
        _containerName = containerName;
        _children = children;
    }

    private static string ResolveSourceDir(string sourceDir)
    {
        if (sourceDir.Length == 0 || sourceDir[0] != '~')
            return sourceDir;
        if (sourceDir.Length == 1 || sourceDir[1] == '/' || sourceDir[1] == '\\')
        {
            var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            return home + sourceDir.Substring(1);
        }
        return sourceDir;
    }

    public string ForFfi()
    {
        var config = new LocalstackConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
            ExpirySeconds = ExpirySeconds,
        };

        if (_services.Count > 0) config.Services = _services;
        if (_queues.Count > 0) config.Queues = _queues;
        if (_lambdas.Count > 0)
        {
            config.Lambdas = _lambdas.Select(spec => new LocalstackLambdaConfig
            {
                Name = spec.Name,
                Runtime = spec.Runtime,
                Handler = spec.Handler,
                SourceDir = Path.GetFullPath(ResolveSourceDir(spec.SourceDir)),
                Environment = spec.Environment.Select(kv => new List<string> { kv.Key, kv.Value }).ToList(),
            }).ToList();
        }
        if (_eventBuses.Count > 0) config.EventBuses = _eventBuses;
        if (_eventRules.Count > 0) config.EventRules = _eventRules;
        if (!string.IsNullOrEmpty(_imageName)) config.ImageName = _imageName;
        if (!string.IsNullOrEmpty(_imageTag)) config.ImageTag = _imageTag;
        if (!string.IsNullOrEmpty(_containerName)) config.ContainerName = _containerName;
        config.Children = ChildrenWireFormat.Build(_children);

        return ArenaJson.Serialize(config);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class LocalstackConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
        [JsonProperty("expiry_seconds")] public long? ExpirySeconds { get; set; }
        [JsonProperty("services")] public List<string>? Services { get; set; }
        [JsonProperty("queues")] public List<LocalstackQueueConfig>? Queues { get; set; }
        [JsonProperty("lambdas")] public List<LocalstackLambdaConfig>? Lambdas { get; set; }
        [JsonProperty("event_buses")] public List<LocalstackEventBusConfig>? EventBuses { get; set; }
        [JsonProperty("event_rules")] public List<LocalstackRuleConfig>? EventRules { get; set; }
        [JsonProperty("image_name")] public string? ImageName { get; set; }
        [JsonProperty("image_tag")] public string? ImageTag { get; set; }
        [JsonProperty("container_name")] public string? ContainerName { get; set; }
        [JsonProperty("children")] public List<JToken>? Children { get; set; }
    }
}

public sealed class LocalstackDependencyBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private int _port = 4566;
    private readonly List<string> _services = new();
    private readonly List<QueueSpec> _queues = new();
    private readonly List<LambdaSpec> _lambdas = new();
    private readonly List<string> _eventBuses = new();
    private readonly List<EventRuleSpec> _eventRules = new();
    private string? _imageName;
    private string? _imageTag;
    private string? _containerName;
    private readonly List<IArenaDependency> _children = new();

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

    public LocalstackDependencyBuilder WithLambda(LambdaSpec spec)
    {
        _lambdas.Add(spec);
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

    public LocalstackDependencyBuilder WithImageName(string imageName)
    {
        _imageName = imageName;
        return this;
    }

    public LocalstackDependencyBuilder WithImageTag(string imageTag)
    {
        _imageTag = imageTag;
        return this;
    }

    public LocalstackDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public LocalstackDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public LocalstackDependencyBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public LocalstackDependencyBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
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

        var built = new LocalstackDependency(
            identifier,
            _port,
            new List<string>(_services),
            queues,
            new List<LambdaSpec>(_lambdas),
            eventBuses,
            rules,
            _imageName,
            _imageTag,
            _containerName,
            _children);
        built.ExpirySeconds = _expirySeconds;
        return built;
    }

    private static long ExpirySeconds(System.TimeSpan expiry)
    {
        if (expiry < System.TimeSpan.Zero)
            throw new System.ArgumentOutOfRangeException(nameof(expiry), "expiry must not be negative");
        var seconds = (long)expiry.TotalSeconds;
        return seconds == 0 && expiry > System.TimeSpan.Zero ? 1 : seconds;
    }

}
