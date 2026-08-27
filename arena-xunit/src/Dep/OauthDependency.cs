using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public sealed class OauthDependency : IArenaDependency
{
    public string Type => "oauth";
    public string Identifier { get; }
    public int Port { get; }
    public string? ListenIp { get; }
    public string? MetadataBaseUrl { get; }
    public string? Transport { get; }
    [JsonProperty("server_tls_certificate_pem")] public string? ServerTlsCert { get; }
    [JsonProperty("server_tls_private_key_pem")] public string? ServerTlsKey { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);
    [JsonProperty("issuers")] public List<JObject>? Issuers { get; }

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal OauthDependency(string identifier, int port, string? listenIp, string? metadataBaseUrl,
        string? transport, string? serverTlsCert, string? serverTlsKey, IReadOnlyList<IArenaDependency> children,
        List<JObject>? issuers)
    {
        Identifier = identifier;
        Port = port;
        ListenIp = listenIp;
        MetadataBaseUrl = metadataBaseUrl;
        Transport = transport;
        ServerTlsCert = serverTlsCert;
        ServerTlsKey = serverTlsKey;
        _children = children;
        Issuers = issuers;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }

    public string SignClaims(OpenArena arena, uint issuerIndex, string claimsJson)
    {
        arena.ThrowIfDisposed();
        return ArenaBindings.OauthSignClaims(arena.Handle, Identifier, issuerIndex, claimsJson);
    }
}

public sealed class OauthDependencyBuilder
{
    public const int DefaultOauthPort = 9444;

    public static readonly string OauthIssuer = IssuerFromEnv();

    private static string IssuerFromEnv()
    {
        var v = Environment.GetEnvironmentVariable("ARENA_PYTEST_OAUTH_ISSUER");
        if (!string.IsNullOrWhiteSpace(v))
        {
            return v.Trim().TrimEnd('/');
        }
        return $"https://127.0.0.1:{DefaultOauthPort}";
    }

    private readonly string _name;
    private int _port = DefaultOauthPort;
    private string? _listenIp;
    private string? _metadataBaseUrl;
    private string? _transport;
    private string? _serverTlsCert;
    private string? _serverTlsKey;
    private readonly List<IArenaDependency> _children = new();
    private readonly List<JObject> _issuers = new();

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
        _metadataBaseUrl = url.TrimEnd('/');
        return this;
    }

    public OauthDependencyBuilder WithHttp()
    {
        _transport = "http";
        return this;
    }

    public OauthDependencyBuilder WithServerTlsPem(string cert, string key)
    {
        _serverTlsCert = cert;
        _serverTlsKey = key;
        _transport = "tls";
        return this;
    }

    public OauthDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public OauthDependencyBuilder WithIssuerCognito(string poolId)
    {
        _issuers.Add(new JObject { ["provider"] = "cognito", ["pool_id"] = poolId });
        return this;
    }

    public OauthDependencyBuilder WithIssuerOkta()
    {
        _issuers.Add(new JObject { ["provider"] = "okta" });
        return this;
    }

    public OauthDependencyBuilder WithIssuerEntraId(string tenantId)
    {
        _issuers.Add(new JObject { ["provider"] = "entra_id", ["tenant_id"] = tenantId });
        return this;
    }

    public OauthDependencyBuilder WithIssuer(string? issuerPath = null, string? jwksPath = null, string? rsaPkcs8Pem = null)
    {
        var entry = new JObject { ["provider"] = "custom" };
        if (issuerPath != null)
            entry["issuer_path"] = issuerPath;
        if (jwksPath != null)
            entry["jwks_path"] = jwksPath;
        if (rsaPkcs8Pem != null)
            entry["rsa_pkcs8_pem"] = rsaPkcs8Pem;
        _issuers.Add(entry);
        return this;
    }

    public OauthDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-oauth", _name);
        var metadataBaseUrl = string.IsNullOrWhiteSpace(_metadataBaseUrl) ? OauthIssuer : _metadataBaseUrl;
        var issuers = _issuers.Count > 0 ? _issuers : null;
        return new OauthDependency(identifier, _port, _listenIp, metadataBaseUrl, _transport, _serverTlsCert, _serverTlsKey, _children, issuers);
    }
}
