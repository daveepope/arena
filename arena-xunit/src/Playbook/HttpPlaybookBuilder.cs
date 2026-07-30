using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json;

namespace ArenaXunit.Playbook;

public sealed class HttpPlaybookBuilder
{
    private readonly string _dependencyIdentifier;
    private readonly List<object> _mappings = new();

    public HttpPlaybookBuilder(string dependencyIdentifier)
    {
        _dependencyIdentifier = dependencyIdentifier;
    }

    public HttpMappingBuilder Get(string path)
    {
        return new HttpMappingBuilder(_dependencyIdentifier, "GET", path, _mappings);
    }

    public HttpMappingBuilder Post(string path)
    {
        return new HttpMappingBuilder(_dependencyIdentifier, "POST", path, _mappings);
    }

    public HttpMappingBuilder Put(string path)
    {
        return new HttpMappingBuilder(_dependencyIdentifier, "PUT", path, _mappings);
    }

    public HttpMappingBuilder Delete(string path)
    {
        return new HttpMappingBuilder(_dependencyIdentifier, "DELETE", path, _mappings);
    }

    internal List<object> BuildMappings()
    {
        return new List<object>(_mappings);
    }
}

public sealed class HttpMappingBuilder
{
    private readonly string _dependencyIdentifier;
    private readonly string _method;
    private readonly string _path;
    private readonly List<object> _mappings;
    private readonly List<HttpResponseObj> _responses = new();
    private int? _expectCalled;
    private bool _expectNeverCalled;

    internal HttpMappingBuilder(string dependencyIdentifier, string method, string path, List<object> mappings)
    {
        _dependencyIdentifier = dependencyIdentifier;
        _method = method;
        _path = path;
        _mappings = mappings;
    }

    public HttpMappingBuilder WillReturn(HttpResponseObj response)
    {
        _responses.Add(response);
        return this;
    }

    public HttpMappingBuilder ThenReturn(HttpResponseObj response)
    {
        return WillReturn(response);
    }

    public List<object> BuildMappings()
    {
        CommitMapping();
        return new List<object>(_mappings);
    }

    public HttpMappingBuilder ExpectCalled(int count)
    {
        _expectCalled = count;
        CommitMapping();
        return this;
    }

    public HttpMappingBuilder ExpectNeverCalled()
    {
        _expectNeverCalled = true;
        CommitMapping();
        return this;
    }

    private void CommitMapping()
    {
        if (_mappings.Any(m =>
        {
            var mc = m as MappingConfig;
            return mc != null && mc.Method == _method && mc.Path == _path;
        }))
            return;

        ExpectConfig? expect = null;
        if (_expectCalled.HasValue)
        {
            expect = new ExpectConfig { Kind = "exactly", Count = _expectCalled.Value };
        }
        else if (_expectNeverCalled)
        {
            expect = new ExpectConfig { Kind = "never" };
        }

        _mappings.Add(new MappingConfig
        {
            DependencyIdentifier = _dependencyIdentifier,
            Method = _method,
            Path = _path,
            Responses = _responses,
            Expect = expect,
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class MappingConfig
    {
        [JsonProperty("dependency_identifier")] public string? DependencyIdentifier { get; set; }
        [JsonProperty("method")] public string? Method { get; set; }
        [JsonProperty("url_path")] public string? Path { get; set; }
        [JsonProperty("responses")] public List<HttpResponseObj>? Responses { get; set; }
        [JsonProperty("expect")] public ExpectConfig? Expect { get; set; }
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class ExpectConfig
    {
        [JsonProperty("kind")] public string? Kind { get; set; }
        [JsonProperty("count")] public int? Count { get; set; }
    }
}
