using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public sealed class SmtpDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "smtp";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;
    public int UiPort => (int)_config["ui_port"]!;

    internal SmtpDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class SmtpDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public SmtpDependencyBuilder(string name)
    {
        _config["type"] = "smtp";
        _config["identifier"] = ArenaIdentifiers.Build("arena-smtp", name);
        _config["port"] = 1025;
        _config["ui_port"] = 8025;
    }

    public SmtpDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }
    public SmtpDependencyBuilder WithUiPort(int uiPort) { _config["ui_port"] = uiPort; return this; }
    public SmtpDependencyBuilder WithStarttls() { _config["tls_mode"] = "starttls"; return this; }
    public SmtpDependencyBuilder WithImplicitTls() { _config["tls_mode"] = "implicit_tls"; return this; }
    public SmtpDependencyBuilder WithImage(string image) { _config["image"] = image; return this; }
    public SmtpDependencyBuilder WithContainerName(string containerName) { _config["container_name"] = containerName; return this; }
    public SmtpDependency Build() => new SmtpDependency((JObject)_config.DeepClone());
}