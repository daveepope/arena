using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public sealed class OauthDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "oauth";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;
    public string? ListenIp => (string?)_config["listen_ip"];
    public string? MetadataBaseUrl => (string?)_config["metadata_base_url"];
    public string? ServerTlsCert => (string?)_config["server_tls_certificate_pem"];
    public string? ServerTlsKey => (string?)_config["server_tls_private_key_pem"];

    internal OauthDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class OauthDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public OauthDependencyBuilder(string name)
    {
        _config["type"] = "oauth";
        _config["identifier"] = ArenaIdentifiers.Build("arena-oauth", name);
        _config["port"] = 9443;
    }

    public OauthDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }
    public OauthDependencyBuilder WithListenIp(string listenIp) { _config["listen_ip"] = listenIp; return this; }
    public OauthDependencyBuilder WithMetadataBaseUrl(string url) { _config["metadata_base_url"] = url; return this; }
    public OauthDependencyBuilder WithServerTlsPem(string cert, string key)
    {
        _config["server_tls_certificate_pem"] = cert;
        _config["server_tls_private_key_pem"] = key;
        return this;
    }
    public OauthDependency Build() => new OauthDependency((JObject)_config.DeepClone());
}