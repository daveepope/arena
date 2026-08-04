using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Dep;

public sealed class OauthDependency : IArenaMatchPiece
{
    public string Type => "oauth";
    public string Identifier { get; }
    public int Port { get; }
    public string? ListenIp { get; }
    public string? MetadataBaseUrl { get; }
    [JsonProperty("server_tls_certificate_pem")] public string? ServerTlsCert { get; }
    [JsonProperty("server_tls_private_key_pem")] public string? ServerTlsKey { get; }

    internal OauthDependency(string identifier, int port, string? listenIp, string? metadataBaseUrl,
        string? serverTlsCert, string? serverTlsKey)
    {
        Identifier = identifier;
        Port = port;
        ListenIp = listenIp;
        MetadataBaseUrl = metadataBaseUrl;
        ServerTlsCert = serverTlsCert;
        ServerTlsKey = serverTlsKey;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class OauthDependencyBuilder
{
    private readonly string _name;
    private int _port = 9443;
    private string? _listenIp;
    private string? _metadataBaseUrl;
    private string? _serverTlsCert;
    private string? _serverTlsKey;

    public OauthDependencyBuilder(string name)
    {
        _name = name;
    }

    public OauthDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public OauthDependencyBuilder WithListenIp(string listenIp)
    {
        _listenIp = listenIp;
        return this;
    }

    public OauthDependencyBuilder WithMetadataBaseUrl(string url)
    {
        _metadataBaseUrl = url;
        return this;
    }

    public OauthDependencyBuilder WithServerTlsPem(string cert, string key)
    {
        _serverTlsCert = cert;
        _serverTlsKey = key;
        return this;
    }

    public OauthDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-oauth", _name);
        return new OauthDependency(identifier, _port, _listenIp, _metadataBaseUrl, _serverTlsCert, _serverTlsKey);
    }
}
