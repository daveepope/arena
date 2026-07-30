using System;
using System.IO;
using System.Net.Http.Json;
using System.Text.Json;
using System.Threading.Tasks;
using Amazon.SQS;
using Amazon.SQS.Model;
using ArenaExamples.Test.Shared;
using ArenaXunit;
using ArenaXunit.Dep;
using ArenaXunit.Playbook;
using ArenaXunit.Xunit;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace ArenaExamples.ComponentTest;

public class AspNetTestTopology : IArenaTopology
{
    private static readonly EphemeralTestRuntime Rt = new EphemeralTestRuntime();

    public Match.Match Configure()
    {
        var webAppPort = Rt.AllocatePort();
        var postgresPort = Rt.AllocatePort();
        var mssqlPort = Rt.AllocatePort();
        var calibrationPort = Rt.AllocatePort();
        var localstackPort = Rt.AllocatePort();
        var temporalPort = Rt.AllocatePort();
        var smtpPort = Rt.AllocatePort();
        var oauthPort = Rt.AllocatePort();

        Environment.SetEnvironmentVariable("WEB_APP_PORT", webAppPort.ToString());
        Environment.SetEnvironmentVariable("CALIBRATION_URL", $"http://127.0.0.1:{calibrationPort}");
        Environment.SetEnvironmentVariable("TEMPORAL_TARGET", $"127.0.0.1:{temporalPort}");
        Environment.SetEnvironmentVariable("AWS_ENDPOINT_URL", $"http://127.0.0.1:{localstackPort}");
        Environment.SetEnvironmentVariable("SMTP_HOST", "127.0.0.1");
        Environment.SetEnvironmentVariable("SMTP_PORT", smtpPort.ToString());
        Environment.SetEnvironmentVariable("EVENT_BUS_NAME", "example-api-events");
        Environment.SetEnvironmentVariable("EVENT_SOURCE", "readings.api");

        var postgresConn = $"Host=127.0.0.1;Port={postgresPort};Database=readings_db;Username=readings_user;Password=readings_password";
        Environment.SetEnvironmentVariable("POSTGRES_CONNECTION_STRING", postgresConn);

        var mssqlConn = $"Server=127.0.0.1,{mssqlPort};Database=validationDb;User Id=sa;Password=yourStrong(!)Password;TrustServerCertificate=True";
        Environment.SetEnvironmentVariable("MSSQL_CONNECTION_STRING", mssqlConn);

        var tlsPemPath = WriteTempCaPem();
        Environment.SetEnvironmentVariable("OAUTH_ISSUER_URL", $"https://127.0.0.1:{oauthPort}");
        Environment.SetEnvironmentVariable("OAUTH_TLS_CA_FILE", tlsPemPath);
        Environment.SetEnvironmentVariable("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES", "readings");

        var calibration = new HttpDependencyBuilder("example-api-calibration")
            .WithPort(calibrationPort)
            .Build();

        var postgres = new PostgresDependencyBuilder("example-api-postgres")
            .WithPort(postgresPort)
            .Build();

        var mssql = new MssqlDependencyBuilder("example-api-mssql")
            .WithPort(mssqlPort)
            .Build();

        var oauth = new OauthDependencyBuilder("example-api-oauth")
            .WithPort(oauthPort)
            .Build();

        var localstack = new LocalstackDependencyBuilder("example-api-localstack")
            .WithPort(localstackPort)
            .Build();

        var temporal = new TemporalDependencyBuilder("example-api-temporal")
            .WithPort(temporalPort)
            .Build();

        var smtp = new SmtpDependencyBuilder("example-api-smtp")
            .WithPort(smtpPort)
            .Build();

        var happyPath = new CalibrationApiHappyPathPlaybook(calibration.Identifier, "/api/v1/validate");

        return new Match.MatchBuilder("example-api-happy-path")
            .AddDependency(oauth)
            .AddDependency(postgres)
            .AddDependency(mssql)
            .AddDependency(calibration)
            .AddDependency(localstack)
            .AddDependency(temporal)
            .AddDependency(smtp)
            .RegisterPlaybook(happyPath, execOnDependencyStart: true)
            .Build();
    }

    private static string WriteTempCaPem()
    {
        var path = Path.GetTempFileName();
        File.WriteAllText(path, "-----BEGIN CERTIFICATE-----\nMIIBkTCB+wIJAMbEYQbQ0L8zMA0GCSqGSIb3DQEBCwUAMBMxETAPBgNVBAMMCFRl\nc3RDQS0xMB4XDTE3MDEwMTAwMDAwMFoXDTQ5MTIzMTIzNTk1OVowEzERMA8GA1UE\nAwwIVGVzdCBDQTEwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQVqY8qXQxvQYwJ\nHqPbFhLJqGvVvMxqVqHvMqEwqGvVvMxqVqHvMqEwqGvVvMxqVqHvMqEwqGvVvMxq\nVqHvMqEwo1cwVTAdBgNVHQ4EFgQU0Z3YS5RVqY8qXQxvQYwJHqPbFhLJqG8wHwYD\nVR0jBBgwFoAU0Z3YS5RVqY8qXQxvQYwJHqPbFhLJqG8wDwYDVR0TAQH/BAUwAwEB\n/zANBgkqhkiG9w0BAQsFAANBAJWqY8qXQxvQYwJHqPbFhLJqGvVvMxqVqHvMqEwq\nGvVvMxqVqHvMqEwqGvVvMxqVqHvMqEwqGvVvMxqVqHvMqE=\n-----END CERTIFICATE-----");
        return path;
    }
}

[Collection("AspnetArena")]
public class AspNetComponentTest : IClassFixture<ArenaCollectionFixture<AspNetTestTopology>>
{
    private readonly OpenArena _arena;
    private readonly ApiClient _client;
    private readonly int _webAppPort;
    private readonly int _calibrationPort;

    public AspNetComponentTest(ArenaCollectionFixture<AspNetTestTopology> fixture)
    {
        _arena = fixture.Arena;

        _webAppPort = int.Parse(Environment.GetEnvironmentVariable("WEB_APP_PORT")!);
        _calibrationPort = int.Parse(Environment.GetEnvironmentVariable("CALIBRATION_URL")!
            .Replace("http://127.0.0.1:", ""));

        var token = FetchAccessTokenAsync().GetAwaiter().GetResult();
        _client = new ApiClient($"http://127.0.0.1:{_webAppPort}", token);
    }

    private static async Task<string> FetchAccessTokenAsync()
    {
        var issuerUrl = Environment.GetEnvironmentVariable("OAUTH_ISSUER_URL")!;
        using var handler = new HttpClientHandler();
        var caFile = Environment.GetEnvironmentVariable("OAUTH_TLS_CA_FILE");
        if (!string.IsNullOrEmpty(caFile))
        {
            var cert = new System.Security.Cryptography.X509Certificates.X509Certificate2(caFile);
            handler.ServerCertificateCustomValidationCallback = (msg, cert2, chain, err) =>
                err == System.Net.Security.SslPolicyErrors.None || cert2 == cert;
        }

        using var http = new HttpClient(handler);
        var discovery = await http.GetFromJsonAsync<DiscoveryDoc>($"{issuerUrl}/.well-known/openid-configuration");
        var tokenEndpoint = discovery?.TokenEndpoint ?? $"{issuerUrl}/token";

        var form = new System.Collections.Generic.Dictionary<string, string>
        {
            { "grant_type", "client_credentials" },
            { "client_id", "arena-examples" },
            { "scope", "readings" }
        };
        var content = new FormUrlEncodedContent(form);
        var response = await http.PostAsync(tokenEndpoint, content);
        response.EnsureSuccessStatusCode();
        var tokenResponse = await response.Content.ReadFromJsonAsync<TokenResponse>();
        return tokenResponse?.AccessToken ?? throw new InvalidOperationException("No access token");
    }

    [Fact]
    public async Task health_endpoint_returns_ok()
    {
        var response = await _client.GetAsync("/health");
        Assert.Equal("ok", response.Trim());
    }

    [Fact]
    public async Task get_readings_returns_list()
    {
        var readings = await _client.GetJsonAsync<List<ReadingDto>>("/readings");
        Assert.NotNull(readings);
    }

    [Fact]
    public async Task get_devices_returns_list()
    {
        var devices = await _client.GetJsonAsync<List<DeviceDto>>("/devices");
        Assert.NotNull(devices);
    }

    private class DiscoveryDoc
    {
        public string? TokenEndpoint { get; set; }
    }

    private class TokenResponse
    {
        public string? AccessToken { get; set; }
    }

    private class ReadingDto
    {
        public int Id { get; set; }
        public string UserName { get; set; } = "";
        public int Value { get; set; }
        public string? Comment { get; set; }
    }

    private class DeviceDto
    {
        public int Id { get; set; }
        public string Name { get; set; } = "";
    }
}
