using System;
using System.Linq;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class OauthDependencyBuilderSerializationTest
{
    [Fact]
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new OauthDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("oauth", obj["type"]);
        Assert.Equal(9444, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_NoMetadataBaseUrl_DefaultsToOauthIssuer()
    {
        var dep = new OauthDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(OauthDependencyBuilder.OauthIssuer, obj["metadata_base_url"]);
    }

    [Fact]
    public void WithMetadataBaseUrl_TrailingSlashes_Stripped()
    {
        var dep = new OauthDependencyBuilder("test").WithMetadataBaseUrl("https://issuer.example.com///").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("https://issuer.example.com", obj["metadata_base_url"]);
    }

    [Fact]
    public void Build_WithHttp_SerializesCorrectJson()
    {
        var dep = new OauthDependencyBuilder("test").WithHttp().Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("http", obj["transport"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new OauthDependencyBuilder("test").WithPort(9543).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(9543, obj["port"]);
    }

    [Fact]
    public void Build_WithListenIp_SerializesCorrectJson()
    {
        var dep = new OauthDependencyBuilder("test").WithListenIp("0.0.0.0").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("0.0.0.0", obj["listen_ip"]);
    }

    [Fact]
    public void Build_WithMetadataBaseUrl_SerializesCorrectJson()
    {
        var dep = new OauthDependencyBuilder("test").WithMetadataBaseUrl("https://example.com/.well-known").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("https://example.com/.well-known", obj["metadata_base_url"]);
    }

    [Fact]
    public void Build_WithServerTlsPem_SerializesCorrectJson()
    {
        var dep = new OauthDependencyBuilder("test").WithServerTlsPem("cert", "key").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("cert", obj["server_tls_certificate_pem"]);
        Assert.Equal("key", obj["server_tls_private_key_pem"]);
        Assert.Equal("tls", obj["transport"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new OauthDependencyBuilder("test").Build();
        Assert.StartsWith("arena-oauth-", dep.Identifier);
    }
}
