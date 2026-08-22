using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Security.Cryptography.X509Certificates;
using System.Text.Json;
using ArenaExamples.Test.Shared;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Ffi;
using Microsoft.Extensions.Logging;

namespace ArenaExamples.ComponentTest;

public sealed class ChainedExampleFixture : ArenaCollectionFixture
{
    public const string QueueName = "readings-events-chained-queue";
    private const string EventBusName = "example-api-chained-events";
    private const string EventSource = "readings.api";
    private const string EventRuleName = "readings-event-chained-rule";
    private const string Region = "us-east-1";

    private const string PostgresHost = "127.0.0.1";
    private const string MssqlHost = "127.0.0.1";

    public static int WebAppPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int WebAppChildPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int PostgresPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int MssqlPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int OraclePort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int CalibrationPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int LocalstackPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int TemporalPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int TemporalUiPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int SmtpPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int SmtpUiPort { get; } = EphemeralTestRuntime.AllocatePort();
    public static int OauthPort { get; } = EphemeralTestRuntime.AllocatePort();

    [ArenaLogger(Level = ArenaLogLevel.Debug)]
    private static readonly ILogger Log =
        LoggerFactory.Create(builder => builder.SetMinimumLevel(LogLevel.Debug).AddConsole())
            .CreateLogger(nameof(ChainedExampleFixture));

    private static readonly OauthLoopbackTlsPemPair OauthPem = OauthLoopbackTls.OauthLoopbackTlsPemPair();

    private const string PostgresDbName = "testdb";
    private const string PostgresDbUser = "test";
    private const string PostgresDbPassword = "test";
    private const string MssqlDbName = "testdb";
    private const string MssqlDbUser = "sa";
    private const string MssqlDbPassword = "Password123!";
    private const string OracleDbName = "weatherdb";
    private const string OracleDbUser = "weather_user";
    private const string OracleDbPassword = "weatherPassword1";
    private const string OracleAdminPassword = "weatherAdminPass1";

    private static readonly PostgresDependency Postgres =
        new PostgresDependencyBuilder("test-chained-postgres")
            .WithPort(PostgresPort)
            .WithDatabaseName(PostgresDbName)
            .WithDatabaseUsername(PostgresDbUser)
            .WithDatabasePassword(PostgresDbPassword)
            .WithStartupSqlScripts(new[] { ResolveSchemaScript("instrument_reading_db_schema.sql") })
            .Build();

    [ArenaDependency]
    private static readonly MssqlDependency Mssql =
        new MssqlDependencyBuilder("test-chained-mssql")
            .WithPort(MssqlPort)
            .WithDatabaseName(MssqlDbName)
            .WithDatabaseUsername(MssqlDbUser)
            .WithDatabasePassword(MssqlDbPassword)
            .WithStartupSqlScripts(new[] { ResolveSchemaScript("validation_db_schema.sql") })
            .Build();

    [ArenaDependency]
    private static readonly OracleDependency Oracle =
        new OracleDependencyBuilder("test-chained-oracle")
            .WithPort(OraclePort)
            .WithDatabaseName(OracleDbName)
            .WithDatabaseUsername(OracleDbUser)
            .WithDatabasePassword(OracleDbPassword)
            .WithAdminPassword(OracleAdminPassword)
            .WithStartupSqlScripts(new[] { ResolveSchemaScript("weather_db_schema.sql") })
            .Build();

    [ArenaDependency]
    private static readonly LocalstackDependency Localstack =
        new LocalstackDependencyBuilder("test-chained-localstack")
            .WithPort(LocalstackPort)
            .WithServices("sqs", "events")
            .WithQueue(QueueName)
            .WithEventBus(EventBusName)
            .WithEventRule(new EventRuleSpec
            {
                Name = EventRuleName,
                EventBus = EventBusName,
                EventPattern = JsonSerializer.Serialize(new { source = new[] { EventSource } }),
                Targets = new List<EventRuleTarget>
                {
                    EventRuleTargetBuilder.SqsQueue("target-queue", QueueName),
                },
            })
            .Build();

    [ArenaDependency]
    private static readonly TemporalDependency Temporal =
        new TemporalDependencyBuilder("test-chained-temporal")
            .WithPort(TemporalPort)
            .WithUiPort(TemporalUiPort)
            .AddChildDependency(Postgres)
            .Build();

    [ArenaDependency]
    private static readonly OauthDependency Oauth =
        new OauthDependencyBuilder("test-chained-oauth")
            .WithPort(OauthPort)
            .WithServerTlsPem(OauthPem.CertificatePem, OauthPem.PrivateKeyPem)
            .WithMetadataBaseUrl($"https://127.0.0.1:{OauthPort}")
            .Build();

    [ArenaDependency]
    private static readonly SmtpDependency Smtp =
        new SmtpDependencyBuilder("test-chained-smtp").WithPort(SmtpPort).WithUiPort(SmtpUiPort).WithStarttls().Build();

    [ArenaDependency]
    private static readonly HttpDependency Calibration =
        new HttpDependencyBuilder("test-chained-calibration").WithPort(CalibrationPort).Build();

    [ArenaPlaybook(ExecOnDependencyStart = true)]
    private static readonly Playbooks.CalibrationHappyPathPlaybook CalibrationHappyPath =
        new(Calibration.Identifier);

    [ArenaPlaybook(ExecOnDependencyStart = true)]
    private static readonly Playbooks.EventsPurgePlaybook EventsPurge =
        new(Localstack.Identifier);

    [ArenaPlaybook(ExecOnDependencyStart = false)]
    private static readonly Playbooks.ResetReadingsDbPlaybook ResetReadingsDb =
        new(Postgres.Identifier);

    [ArenaPlaybook(ExecOnDependencyStart = false)]
    private static readonly Playbooks.ResetWeatherDbPlaybook ResetWeatherDb =
        new(Oracle.Identifier);

    private static readonly ExecutableComponent WebAppChild =
        BuildWebApp("example-api-chained-web-app-child", WebAppChildPort, Array.Empty<IArenaComponent>());

    [ArenaComponent(Logs = false)]
    private static readonly ExecutableComponent WebApp =
        BuildWebApp("example-api-chained-web-app", WebAppPort, new IArenaComponent[] { WebAppChild });

    private static ExecutableComponent BuildWebApp(string name, int port, IEnumerable<IArenaComponent> children)
    {
        var builder = new ExecutableComponentBuilder(name)
            .WithExecutablePath(ResolveWebAppExecutablePath())
            .WithEnvVar("WEB_APP_PORT", port.ToString())
            .WithEnvVar("CALIBRATION_URL", $"http://127.0.0.1:{CalibrationPort}")
            .WithEnvVar("POSTGRES_HOST", PostgresHost)
            .WithEnvVar("POSTGRES_PORT", PostgresPort.ToString())
            .WithEnvVar("POSTGRES_DB", PostgresDbName)
            .WithEnvVar("POSTGRES_USER", PostgresDbUser)
            .WithEnvVar("POSTGRES_PASSWORD", PostgresDbPassword)
            .WithEnvVar("POSTGRES_CONNECTION_STRING",
                $"Host={PostgresHost};Port={PostgresPort};Username={PostgresDbUser};Password={PostgresDbPassword};Database={PostgresDbName}")
            .WithEnvVar("MSSQL_HOST", MssqlHost)
            .WithEnvVar("MSSQL_PORT", MssqlPort.ToString())
            .WithEnvVar("MSSQL_DB_NAME", MssqlDbName)
            .WithEnvVar("MSSQL_DB_USER", MssqlDbUser)
            .WithEnvVar("MSSQL_DB_PASSWORD", MssqlDbPassword)
            .WithEnvVar("MSSQL_CONNECTION_STRING",
                $"Server=tcp:{MssqlHost},{MssqlPort};Database={MssqlDbName};User Id={MssqlDbUser};Password={MssqlDbPassword};TrustServerCertificate=True;")
            .WithEnvVar("ORACLE_CONNECTION_STRING",
                $"{OracleDbUser}/{OracleDbPassword}@localhost:{OraclePort}/{OracleDbName}")
            .WithEnvVar("TEMPORAL_HOST", "127.0.0.1")
            .WithEnvVar("TEMPORAL_PORT", TemporalPort.ToString())
            .WithEnvVar("TEMPORAL_TARGET", $"127.0.0.1:{TemporalPort}")
            .WithEnvVar("SMTP_HOST", "127.0.0.1")
            .WithEnvVar("SMTP_PORT", SmtpPort.ToString())
            .WithEnvVar("SMTP_FROM", "test@example.com")
            .WithEnvVar("LOCALSTACK_HOST", $"127.0.0.1:{LocalstackPort}")
            .WithEnvVar("LOCALSTACK_REGION", Region)
            .WithEnvVar("AWS_REGION", Region)
            .WithEnvVar("AWS_ENDPOINT_URL", $"http://127.0.0.1:{LocalstackPort}")
            .WithEnvVar("AWS_DEFAULT_REGION", Region)
            .WithEnvVar("AWS_ACCESS_KEY_ID", "test")
            .WithEnvVar("AWS_SECRET_ACCESS_KEY", "test")
            .WithEnvVar("EVENT_BUS_NAME", EventBusName)
            .WithEnvVar("EVENT_SOURCE", EventSource)
            .WithEnvVar("OAUTH_URL", $"https://127.0.0.1:{OauthPort}")
            .WithEnvVar("OAUTH_ISSUER", $"https://127.0.0.1:{OauthPort}")
            .WithEnvVar("OAUTH_ISSUER_URL", $"https://127.0.0.1:{OauthPort}")
            .WithEnvVar("OAUTH_TLS_CA_FILE", ResolveOauthCaCertFilePath())
            .WithEnvVar("OAUTH_CLIENT_ID", "test-client")
            .WithEnvVar("OAUTH_CLIENT_SECRET", "test-secret")
            .WithEnvVar("LD_LIBRARY_PATH", ResolveTemporalNativeLibDir())
            .WithReadinessCheck(HttpReadinessCheck.Create(), $"http://127.0.0.1:{port}/health");
        foreach (var child in children)
        {
            builder.AddChildComponent(child);
        }
        return builder.Build();
    }

    public ApiClient ApiClient { get; }
    public ApiClient ApiClient2 { get; }

    public ChainedExampleFixture() : base()
    {
        var authToken = GetAuthToken();
        ApiClient = new ApiClient($"http://127.0.0.1:{WebAppPort}", authToken);
        ApiClient2 = new ApiClient($"http://127.0.0.1:{WebAppChildPort}", authToken);
    }

    private static string GetAuthToken()
    {
        var trustedCert = X509Certificate2.CreateFromPem(OauthPem.CertificatePem);
        var handler = new HttpClientHandler
        {
            ServerCertificateCustomValidationCallback = (_, cert, _, _) =>
                cert != null && cert.GetCertHashString() == trustedCert.GetCertHashString(),
        };
        using var client = new HttpClient(handler);
        var content = new Dictionary<string, string>
        {
            ["grant_type"] = "client_credentials",
            ["client_id"] = "test-client",
            ["client_secret"] = "test-secret"
        };
        var response = client.PostAsync($"https://127.0.0.1:{OauthPort}/oauth/token",
            new FormUrlEncodedContent(content)).Result;
        response.EnsureSuccessStatusCode();
        var json = JsonSerializer.Deserialize<JsonElement>(response.Content.ReadAsStringAsync().Result);
        return json.GetProperty("access_token").GetString()!;
    }

    private static string ResolveOauthCaCertFilePath()
    {
        var path = Path.Combine(Path.GetTempPath(), $"arena-example-chained-oauth-ca-{Environment.ProcessId}.pem");
        File.WriteAllText(path, OauthPem.CertificatePem);
        return path;
    }

    private static string ResolveRunfilesRoot()
    {
        var runfilesRoot = Environment.GetEnvironmentVariable("RUNFILES_DIR");
        if (string.IsNullOrEmpty(runfilesRoot))
        {
            var assemblyDir = Path.GetDirectoryName(typeof(ChainedExampleFixture).Assembly.Location);
            var parent = !string.IsNullOrEmpty(assemblyDir) ? Directory.GetParent(assemblyDir!)?.FullName : null;
            if (!string.IsNullOrEmpty(parent) && parent!.Contains(".runfiles"))
                runfilesRoot = parent;
        }

        if (string.IsNullOrEmpty(runfilesRoot))
            throw new InvalidOperationException("could not determine RUNFILES_DIR");
        return runfilesRoot;
    }

    private static string ResolveSchemaScript(string filename)
    {
        var path = FindSingleRunfile(filename);
        return File.ReadAllText(path);
    }

    private static string ResolveWebAppExecutablePath()
    {
        return FindSingleRunfile("example-readings-aspnet-web-app.dll.sh");
    }

    private static string ResolveTemporalNativeLibDir()
    {
        var path = FindSingleRunfile("libtemporalio_sdk_core_c_bridge.so");
        return Path.GetDirectoryName(path)!;
    }

    private static string FindSingleRunfile(string fileName)
    {
        var runfilesRoot = ResolveRunfilesRoot();
        var matches = Directory.GetFiles(runfilesRoot, fileName, SearchOption.AllDirectories);
        if (matches.Length == 0)
            throw new InvalidOperationException($"{fileName} not found under Bazel runfiles at '{runfilesRoot}'");
        if (matches.Length == 1)
            return matches[0];

        var distinctContents = new HashSet<string>();
        foreach (var match in matches)
            distinctContents.Add(Convert.ToBase64String(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(match))));

        if (distinctContents.Count > 1)
        {
            throw new InvalidOperationException(
                $"ambiguous runfile lookup for '{fileName}' under '{runfilesRoot}': found {matches.Length} matches " +
                $"with different contents ({string.Join(", ", matches)}); narrow the search or reference the file directly");
        }
        return matches[0];
    }
}
